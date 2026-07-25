//! JSON HTTP API for the Vue operator UI.
//! Auth is optional and **off by default**; with `--auth` / `[server] auth = true` /
//! `KOWALSKI_API_TOKEN` set, `/api/*` requires a locally generated bearer token and CORS
//! becomes an origin allowlist (see `crate::auth`). `/api/health` always stays open.
//! `/api/chat` and `/api/chat/stream` use one in-process `TemplateAgent` + configured LLM (`[llm]` +
//! `[ollama].model` — Ollama or OpenAI-compatible API).

use axum::extract::Path as AxumPath;
use axum::extract::Query;
use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, patch, post};
use axum::{Extension, Json, Router};
use futures::Stream;
use futures::StreamExt;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
#[cfg(feature = "postgres")]
use kowalski_core::federation::MessageBroker;
use kowalski_core::federation::{
    AclEnvelope, AclMessage, AgentRecord, AgentRegistry, FederationOrchestrator, MpscBroker,
};
use kowalski_core::template::agent::TemplateAgent;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fs::OpenOptions;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

/// Returns the model to use based on the LLM provider configuration.
/// Priority order:
/// 1. llm.model (if set and provider is openai)
/// 2. ollama.model (fallback for both providers)
fn determine_model(config: &Config) -> String {
    // If using openai provider and llm.model is set, use that
    if config.llm.provider == "openai" {
        if let Some(ref model) = config.llm.model {
            return model.clone();
        }
    }
    
    // Fallback to ollama.model for both providers
    config.ollama.model.clone()
}

#[derive(Serialize)]
struct MemoryStatus {
    backend: String,
    episodic_buffer_count: usize,
    embeddings_ok: bool,
    embed_model: String,
    last_embed_error: Option<String>,
}

struct ChatState {
    agent: TemplateAgent,
    conv_id: String,
}

#[derive(Clone)]
struct ApiState {
    config_path: PathBuf,
    ollama_url: Option<String>,
    model: String,
    full_config: Config,
    chat: Arc<Mutex<ChatState>>,
    federation_broker: Arc<MpscBroker>,
    federation: Arc<FederationOrchestrator>,
    managed_workers: Arc<Mutex<HashMap<String, Child>>>,
    managed_worker_last_exit: Arc<Mutex<HashMap<String, String>>>,
    horde_manager: crate::horde::HordeManager,
    /// This server's own base URL (from `--bind` + TLS scheme) — passed to spawned
    /// workers as `--api` so they call back to the right address.
    api_url: String,
    /// Bearer token required on `/api/*` (`None` when auth is off — the default); handed to
    /// spawned workers via `KOWALSKI_API_TOKEN`.
    api_token: Option<Arc<String>>,
    /// Same DB pool as the LISTEN bridge — used to fan out delegates via `NOTIFY`.
    #[cfg(feature = "postgres")]
    federation_pg_notify: Option<Arc<kowalski_core::PgBroker>>,
}

/// Auth/CORS knobs from the CLI (`--auth`, `--cors-origin`); config `[server]`
/// values apply when the flags are not set.
#[derive(Debug, Default, Clone)]
pub struct SecurityOptions {
    pub auth: bool,
    pub cors_origins: Vec<String>,
}

/// Run until SIGINT / process exit. Binds `addr` and serves under `/api/*`.
/// When `tls` is `Some((cert_pem, key_pem))`, serves HTTPS via rustls (`axum-server`).
pub async fn serve(
    addr: SocketAddr,
    config: Option<String>,
    ollama_url: Option<String>,
    tls: Option<(PathBuf, PathBuf)>,
    security: SecurityOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = crate::http_ops::mcp_config_path(config.as_deref());
    let full_config = crate::http_ops::load_kowalski_config_for_serve(&config_path)?;

    let auth_enabled = security.auth
        || server_config_auth(&full_config)
        || std::env::var(crate::auth::TOKEN_ENV)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
    let api_token: Option<Arc<String>> = if auth_enabled {
        let state_root = state_config_dir(&config_path)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("db");
        let (token, generated, token_path) = crate::auth::resolve_api_token(&state_root)?;
        if generated {
            println!(
                "Generated API token (also persisted at {}):\n  {}",
                token_path.display(),
                token
            );
        }
        log::info!(
            "API auth: bearer token required on /api/* (token file: {})",
            token_path.display()
        );
        Some(Arc::new(token))
    } else {
        log::info!(
            "API auth off (default, single-user local mode) — enable with --auth or [server] auth = true"
        );
        None
    };
    let cors_origins = if !security.cors_origins.is_empty() {
        security.cors_origins.clone()
    } else {
        server_config_cors_origins(&full_config).unwrap_or_else(|| {
            crate::auth::DEFAULT_CORS_ORIGINS
                .iter()
                .map(|s| s.to_string())
                .collect()
        })
    };
    kowalski_core::db::run_memory_migrations_if_configured(&full_config).await?;

    let mut agent = TemplateAgent::new(full_config.clone()).await?;
    let model = determine_model(&full_config);
    let conv_id = agent.start_conversation(&model);

    let federation_broker = Arc::new(MpscBroker::new());
    let federation_registry = Arc::new(AgentRegistry::new());
    #[cfg(feature = "postgres")]
    if let Some(ref url) = full_config.memory.database_url
        && kowalski_core::config::memory_uses_postgres(&full_config.memory)
        && let Err(e) = kowalski_core::load_registry_into(&federation_registry, url).await
    {
        log::warn!("federation registry DB load: {}", e);
    }
    let template_agent = AgentRecord {
        id: "template".into(),
        capabilities: vec!["chat".into(), "mcp".into(), "llm".into()],
    };
    federation_registry
        .register(template_agent.clone())
        .map_err(|e| format!("federation registry: {e}"))?;
    #[cfg(feature = "postgres")]
    if let Some(ref url) = full_config.memory.database_url
        && kowalski_core::config::memory_uses_postgres(&full_config.memory)
    {
        if let Err(e) = kowalski_core::upsert_registry_record(url, &template_agent).await {
            log::warn!("federation registry upsert: {}", e);
        }
        if let Err(e) = kowalski_core::upsert_agent_state_for_record(url, &template_agent).await {
            log::warn!("agent_state upsert: {}", e);
        }
    }
    let mut federation =
        FederationOrchestrator::new(federation_registry.clone(), federation_broker.clone());
    federation.orchestrator_id = "kowalski-serve".into();
    federation.default_topic = "federation".into();
    let federation = Arc::new(federation);

    #[cfg(feature = "postgres")]
    let federation_pg_notify = {
        let mut pg_out: Option<Arc<kowalski_core::PgBroker>> = None;
        if kowalski_core::config::memory_uses_postgres(&full_config.memory)
            && let Some(ref url) = full_config.memory.database_url
        {
            match kowalski_core::bridge_postgres_notify_to_mpsc(
                url,
                "kowalski_federation",
                federation_broker.clone(),
            )
            .await
            {
                Ok(pool) => {
                    log::info!(
                        "Federation: Postgres LISTEN kowalski_federation → in-process broker (SSE)"
                    );
                    pg_out = Some(Arc::new(kowalski_core::PgBroker::new(
                        (*pool).clone(),
                        "kowalski_federation",
                    )));
                }
                Err(e) => log::warn!("Federation Postgres bridge: {}", e),
            }
        }
        pg_out
    };

    let scheme = if tls.is_some() { "https" } else { "http" };
    log::info!(
        "Kowalski HTTP API at {}://{} (config {}, model {})",
        scheme,
        addr,
        config_path.display(),
        model
    );

    let horde_roots = crate::horde::default_horde_roots(state_config_dir(&config_path).as_deref());
    let horde_specs = crate::horde::discover_hordes(&horde_roots);
    log::info!(
        "horde catalog: {} horde(s) discovered ({:?})",
        horde_specs.len(),
        horde_specs.iter().map(|s| &s.id).collect::<Vec<_>>()
    );
    let run_store = {
        let state_root = state_config_dir(&config_path)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("db");
        kowalski_core::db::run_store::RunStore::open_default(&state_root)
            .await
            .map_err(|e| format!("run store: {e}"))?
    };
    let mut horde_manager = crate::horde::HordeManager::new(
        horde_specs,
        federation_broker.clone(),
        federation.clone(),
        run_store,
    );
    if let Some(cap) = horde_config_resume_max_attempts(&full_config) {
        horde_manager.resume_max_attempts = cap;
    }
    if let Some(secs) = horde_config_step_timeout_secs(&full_config) {
        horde_manager.step_timeout = std::time::Duration::from_secs(secs);
    }
    // LLM step kinds run in-process by default: a dedicated horde agent (its own
    // conversations; interactive chat keeps its own agent) shared by all LLM
    // step handlers. Worker spawn remains available as the future isolation mode.
    {
        let horde_agent = TemplateAgent::new(full_config.clone()).await?;
        let horde_agent = Arc::new(Mutex::new(horde_agent));
        let mut registry =
            kowalski_core::StepHandlerRegistry::with_builtin_deterministic();
        kowalski_core::LlmStepHandler::register_all(&mut registry, horde_agent, &model);
        horde_manager.step_handlers = Arc::new(registry);
    }
    let horde_manager = horde_manager;
    let global_clean_on_startup = global_horde_clean_on_startup(&full_config);
    // Hordes with interrupted runs pending resume keep their workdir: cleaning
    // would destroy completed steps' artifacts and break `@step:` context
    // attachments after resume.
    let hordes_with_incomplete_runs: std::collections::HashSet<String> = horde_manager
        .store
        .incomplete_runs()
        .await
        .map(|runs| runs.into_iter().map(|r| r.horde_id).collect())
        .unwrap_or_default();
    for spec in horde_manager.specs.iter() {
        let mut effective_clean_on_startup =
            global_clean_on_startup.unwrap_or(spec.config_on_startup);
        if effective_clean_on_startup && hordes_with_incomplete_runs.contains(&spec.id) {
            log::info!(
                "horde {}: skipping workdir clean — interrupted run(s) pending resume",
                spec.id
            );
            effective_clean_on_startup = false;
        }
        if let Err(e) =
            crate::horde::prepare_workdir_on_startup_with_policy(spec, effective_clean_on_startup)
        {
            log::warn!(
                "horde startup workdir prepare failed horde={} workdir={} err={}",
                spec.id,
                spec.workdir.display(),
                e
            );
        } else {
            log::info!(
                "horde startup workdir prepared horde={} workdir={} clean_on_startup={} (global_override={:?})",
                spec.id,
                spec.workdir.display(),
                effective_clean_on_startup,
                global_clean_on_startup
            );
        }
    }
    crate::horde::spawn_orchestrator_loop(horde_manager.clone());
    // Reconcile runs interrupted by the previous shutdown ("agents survive a
    // reboot"): auto-resume non-operator runs, surface the rest as resumable.
    {
        let manager = horde_manager.clone();
        tokio::spawn(async move { manager.resume_scan().await });
    }

    let rookery = crate::rookery::new_rookery_store(&full_config, &config_path).await?;

    let state = ApiState {
        config_path,
        ollama_url,
        model,
        full_config: full_config.clone(),
        chat: Arc::new(Mutex::new(ChatState { agent, conv_id })),
        federation_broker: federation_broker.clone(),
        federation,
        managed_workers: Arc::new(Mutex::new(HashMap::new())),
        managed_worker_last_exit: Arc::new(Mutex::new(HashMap::new())),
        horde_manager,
        api_url: format!("{}://{}", scheme, addr),
        api_token: api_token.clone(),
        #[cfg(feature = "postgres")]
        federation_pg_notify,
    };

    let router = Router::new()
        .route("/api/health", get(get_health))
        .route("/api/agents", get(get_agents))
        .route("/api/sessions", get(get_sessions))
        .route("/api/doctor", get(get_doctor))
        .route("/api/models", get(get_models))
        .route("/api/mcp/servers", get(get_mcp_servers))
        .route("/api/mcp/ping", post(post_mcp_ping))
        .route("/api/memory/status", get(get_memory_status))
        .route("/api/chat", post(post_chat))
        .route("/api/chat/stream", post(post_chat_stream))
        .route("/api/chat/reset", post(post_chat_reset))
        .route("/api/chat/sync", post(post_chat_sync))
        .route("/api/chat/messages", get(get_chat_messages))
        .route("/api/system/open-path", post(post_open_path))
        .route("/api/federation/stream", get(get_federation_stream))
        .route("/api/federation/ws", get(get_federation_ws))
        .route("/api/federation/registry", get(get_federation_registry))
        .route("/api/federation/workers", get(get_federation_workers))
        .route(
            "/api/federation/workers/start",
            post(post_federation_worker_start),
        )
        .route(
            "/api/federation/workers/stop",
            post(post_federation_worker_stop),
        )
        .route("/api/hordes", get(get_hordes))
        .route("/api/hordes/{horde_id}", get(get_horde_detail))
        .route("/api/hordes/{horde_id}/workers", get(get_horde_workers))
        .route(
            "/api/hordes/{horde_id}/workers/start",
            post(post_horde_worker_start),
        )
        .route(
            "/api/hordes/{horde_id}/workers/stop",
            post(post_horde_worker_stop),
        )
        .route(
            "/api/hordes/{horde_id}/repair-outputs",
            post(post_horde_repair_outputs),
        )
        .route("/api/hordes/{horde_id}/run", post(post_horde_run))
        .route(
            "/api/hordes/{horde_id}/clean-workdir",
            post(post_horde_clean_workdir),
        )
        .route("/api/hordes/{horde_id}/followup", post(post_horde_followup))
        .route("/api/hordes/{horde_id}/runs", get(get_horde_runs))
        .route(
            "/api/hordes/{horde_id}/runs/{run_id}",
            get(get_horde_run_detail),
        )
        .route(
            "/api/hordes/{horde_id}/runs/{run_id}/resume",
            post(post_horde_run_resume),
        )
        .route(
            "/api/hordes/{horde_id}/runs/{run_id}/cancel",
            post(post_horde_run_cancel),
        )
        .route("/api/federation/register", post(post_federation_register))
        .route(
            "/api/federation/deregister",
            post(post_federation_deregister),
        )
        .route(
            "/api/federation/cleanup-stale",
            post(post_federation_cleanup_stale),
        )
        .route("/api/federation/heartbeat", post(post_federation_heartbeat))
        .route("/api/federation/delegate", post(post_federation_delegate))
        .route("/api/federation/publish", post(post_federation_publish))
        .route("/api/graph/status", get(get_graph_status))
        .route(
            "/api/rookery/sessions",
            get(crate::rookery::list_sessions).post(crate::rookery::post_sessions),
        )
        .route(
            "/api/rookery/sessions/{session_id}",
            get(crate::rookery::get_session).delete(crate::rookery::delete_session),
        )
        .route(
            "/api/rookery/sessions/{session_id}/chat",
            post(crate::rookery::post_chat),
        )
        .route(
            "/api/rookery/sessions/{session_id}/propose",
            post(crate::rookery::post_propose),
        )
        .route(
            "/api/rookery/sessions/{session_id}/give-birth",
            post(crate::rookery::post_give_birth),
        )
        .route(
            "/api/rookery/sessions/{session_id}/save-horde",
            post(crate::rookery::post_save_horde),
        )
        .route(
            "/api/rookery/sessions/{session_id}/penguins/{penguin_name}",
            patch(crate::rookery::patch_penguin),
        )
        .route(
            "/api/rookery/sessions/{session_id}/validate",
            post(crate::rookery::post_validate_draft),
        );
    #[cfg(feature = "postgres")]
    let router = router.route("/api/graph/cypher", post(post_graph_cypher));
    let app = router
        .with_state(state)
        .layer(Extension(rookery))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(false))
                .on_response(DefaultOnResponse::new()),
        );
    let app = if let Some(token) = api_token {
        app.layer(axum::middleware::from_fn(move |req, next| {
            let token = token.clone();
            crate::auth::require_token(token, req, next)
        }))
    } else {
        app
    };
    // CORS outermost so browser preflights (no Authorization header) never hit the auth check.
    let app = app.layer(crate::auth::cors_layer(!auth_enabled, &cors_origins));

    if let Some((cert, key)) = tls {
        let rustls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
        axum_server::bind_rustls(addr, rustls_config)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }
    Ok(())
}

fn federation_postgres_notify_bridge(state: &ApiState) -> bool {
    #[cfg(feature = "postgres")]
    {
        state.federation_pg_notify.is_some()
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        false
    }
}

fn server_config_auth(cfg: &Config) -> bool {
    cfg.additional
        .get("server")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("auth"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn server_config_cors_origins(cfg: &Config) -> Option<Vec<String>> {
    let origins: Vec<String> = cfg
        .additional
        .get("server")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("cors_origins"))
        .and_then(|v| v.as_array())?
        .iter()
        .filter_map(|v| v.as_str().map(ToString::to_string))
        .collect();
    (!origins.is_empty()).then_some(origins)
}

fn global_horde_clean_on_startup(cfg: &Config) -> Option<bool> {
    cfg.additional
        .get("horde")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("clean_on_startup"))
        .and_then(|v| v.as_bool())
}

/// `[horde] resume_max_attempts` in `config.toml` — cap on resume attempts per
/// run before the orchestrator errors it out (default
/// [`crate::horde::DEFAULT_RESUME_MAX_ATTEMPTS`]).
fn horde_config_resume_max_attempts(cfg: &Config) -> Option<u32> {
    cfg.additional
        .get("horde")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("resume_max_attempts"))
        .and_then(|v| v.as_u64())
        .map(|v| v.min(u32::MAX as u64) as u32)
}

/// `[horde] step_timeout_secs` in `config.toml` — wall-clock limit per in-process
/// step (default [`crate::horde::DEFAULT_STEP_TIMEOUT_SECS`]).
fn horde_config_step_timeout_secs(cfg: &Config) -> Option<u64> {
    cfg.additional
        .get("horde")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("step_timeout_secs"))
        .and_then(|v| v.as_u64())
        .filter(|v| *v > 0)
}

async fn get_health(State(state): State<ApiState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "kowalski",
        "version": env!("CARGO_PKG_VERSION"),
        "model": state.model,
        "federation": {
            "agents_registered": state.federation.registry.list().len(),
            "postgres_notify_bridge": federation_postgres_notify_bridge(&state),
        },
    }))
}

/// Single-process `serve`: one template agent (not a federated `AgentRegistry` yet).
async fn get_agents(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let guard = state.chat.lock().await;
    Json(json!({
        "mode": "single_process",
        "agents": [{
            "name": guard.agent.name(),
            "description": guard.agent.description(),
        }],
        "conversation_id": guard.conv_id,
        "model": state.model,
    }))
}

/// Active conversation(s) for this `serve` process (one in-memory session today).
async fn get_sessions(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let guard = state.chat.lock().await;
    Json(json!({
        "mode": "single_process",
        "sessions": [{
            "id": guard.conv_id,
            "model": state.model,
            "agent_name": guard.agent.name(),
        }],
    }))
}

async fn get_doctor(State(state): State<ApiState>) -> Json<crate::http_ops::DoctorJson> {
    Json(crate::http_ops::doctor_json(state.ollama_url.clone(), Some(&state.full_config)).await)
}

async fn get_models(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let ollama_url = state.ollama_url.clone().unwrap_or_else(|| {
        format!(
            "http://{}:{}",
            state.full_config.ollama.host, state.full_config.ollama.port
        )
    });
    let mut models = crate::http_ops::list_ollama_models(&ollama_url).await;
    if !models.iter().any(|m| m == &state.model) {
        models.insert(0, state.model.clone());
    }
    Json(json!({
        "default_model": state.model,
        "models": models,
    }))
}

async fn get_mcp_servers(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::http_ops::McpServerPublic>>, (StatusCode, String)> {
    crate::http_ops::list_mcp_servers_public(&state.config_path)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn post_mcp_ping(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::http_ops::McpPingResult>>, (StatusCode, String)> {
    crate::http_ops::mcp_ping_results(&state.config_path)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_memory_status(
    State(state): State<ApiState>,
) -> Result<Json<MemoryStatus>, (StatusCode, String)> {
    let llm_provider: Arc<dyn kowalski_core::llm::LLMProvider> =
        Arc::new(kowalski_core::llm::OllamaProvider::new(
            &state.full_config.ollama.host,
            state.full_config.ollama.port,
        ));
    let episodic = kowalski_core::memory::episodic::EpisodicBuffer::open(
        &state.full_config.memory,
        llm_provider,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let memories = episodic
        .retrieve_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let missing_embeddings = memories.iter().filter(|m| m.embedding.is_none()).count();

    let ollama_url = state.ollama_url.clone().unwrap_or_else(|| {
        format!(
            "http://{}:{}",
            state.full_config.ollama.host, state.full_config.ollama.port
        )
    });
    let embed_model = "nomic-embed-text".to_string();
    let probe = reqwest::Client::new()
        .post(format!(
            "{}/api/embeddings",
            ollama_url.trim_end_matches('/')
        ))
        .json(&json!({
            "model": embed_model,
            "prompt": "healthcheck",
        }))
        .send()
        .await;

    let (embeddings_ok, embed_error) = match probe {
        Ok(resp) if resp.status().is_success() => (true, None),
        Ok(resp) => {
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            (false, Some(format!("embedding probe failed: {}", text)))
        }
        Err(e) => (false, Some(format!("embedding probe failed: {}", e))),
    };

    let last_embed_error = if !embeddings_ok {
        embed_error
    } else if missing_embeddings > 0 {
        Some(format!(
            "{} memory item(s) are missing embeddings",
            missing_embeddings
        ))
    } else {
        None
    };

    let backend = if state.full_config.memory.database_url.is_some() {
        "postgres_or_external".to_string()
    } else {
        "sqlite".to_string()
    };

    Ok(Json(MemoryStatus {
        backend,
        episodic_buffer_count: memories.len(),
        embeddings_ok,
        embed_model,
        last_embed_error,
    }))
}

#[derive(Deserialize)]
struct ChatBody {
    message: String,
    /// Optional explicit conversation id to target.
    #[serde(default)]
    conversation_id: Option<String>,
    /// When true, include retrieved memory snippets in prompt assembly.
    #[serde(default = "default_true")]
    use_memory: bool,
    /// When false, bypass tool loop and generate plain assistant text.
    #[serde(default = "default_true")]
    use_tools: bool,
    /// Restrict tool loop to these registered tool names (horde stage `tool_ids`).
    #[serde(default)]
    tool_ids: Option<Vec<String>>,
    /// Restrict filesystem tool paths to this directory (operator `project_path`).
    #[serde(default)]
    sandbox_root: Option<String>,
    /// When true, `POST /api/chat/stream` runs the tool loop and streams **only** the first LLM turn after a tool result (final answer); earlier turns are non-streamed like `POST /api/chat`.
    #[serde(default)]
    tools_stream: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
struct ChatResponse {
    reply: String,
    mode: &'static str,
    model: String,
    memory_used: bool,
    memory_source: String,
    memory_items_count: usize,
}

#[derive(Serialize)]
struct ChatMessagesResponse {
    conversation_id: String,
    model: String,
    messages: Vec<kowalski_core::conversation::Message>,
}

#[derive(Deserialize)]
struct ChatMessagesQuery {
    conversation_id: Option<String>,
}

#[derive(Serialize)]
struct ChatResetResponse {
    conversation_id: String,
    model: String,
}

#[derive(Deserialize)]
struct ChatSyncBody {
    #[serde(default)]
    conversation_id: Option<String>,
    messages: Vec<kowalski_core::conversation::Message>,
}

#[derive(Serialize)]
struct ChatSyncResponse {
    conversation_id: String,
    model: String,
    message_count: usize,
}

#[derive(Deserialize)]
struct OpenPathBody {
    path: String,
}

#[derive(Serialize)]
struct OpenPathResponse {
    ok: bool,
    path: String,
}

async fn post_open_path(
    Json(body): Json<OpenPathBody>,
) -> Result<Json<OpenPathResponse>, (StatusCode, String)> {
    let trimmed = body.path.trim();
    if trimmed.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "path is required".into()));
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err((StatusCode::BAD_REQUEST, "path must be absolute".into()));
    }
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, format!("path not found: {}", path.display())));
    }

    let mut cmd: Command;
    #[cfg(target_os = "macos")]
    {
        cmd = Command::new("open");
        cmd.arg(&path);
    }
    #[cfg(target_os = "linux")]
    {
        cmd = Command::new("xdg-open");
        cmd.arg(&path);
    }
    #[cfg(target_os = "windows")]
    {
        cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", &path.to_string_lossy()]);
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            "opening local paths is not supported on this OS".into(),
        ));
    }

    let out = cmd
        .output()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to launch opener: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            if stderr.is_empty() {
                format!("opener exited with status: {}", out.status)
            } else {
                format!("opener failed: {}", stderr)
            },
        ));
    }

    Ok(Json(OpenPathResponse {
        ok: true,
        path: path.to_string_lossy().to_string(),
    }))
}

async fn post_chat_reset(
    State(state): State<ApiState>,
) -> Result<Json<ChatResetResponse>, (StatusCode, String)> {
    let mut guard = state.chat.lock().await;
    let conversation_id = guard.agent.start_conversation(&state.model);
    guard.conv_id = conversation_id.clone();
    log::info!("HTTP chat: new conversation {}", conversation_id);
    Ok(Json(ChatResetResponse {
        conversation_id,
        model: state.model.clone(),
    }))
}

async fn post_chat_sync(
    State(state): State<ApiState>,
    Json(body): Json<ChatSyncBody>,
) -> Result<Json<ChatSyncResponse>, (StatusCode, String)> {
    if body.messages.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "messages must not be empty".to_string(),
        ));
    }
    let mut guard = state.chat.lock().await;
    let conv_id = if let Some(ref cid) = body.conversation_id {
        if guard.agent.get_conversation(cid).is_some() {
            cid.clone()
        } else {
            let created = guard.agent.start_conversation(&state.model);
            guard.conv_id = created.clone();
            created
        }
    } else {
        let created = guard.agent.start_conversation(&state.model);
        guard.conv_id = created.clone();
        created
    };
    guard
        .agent
        .replace_conversation_messages(&conv_id, body.messages.clone())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    guard.conv_id = conv_id.clone();
    Ok(Json(ChatSyncResponse {
        conversation_id: conv_id,
        model: state.model.clone(),
        message_count: body.messages.len(),
    }))
}

async fn post_chat(
    State(state): State<ApiState>,
    Json(body): Json<ChatBody>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let mut guard = state.chat.lock().await;
    let conv_id = if let Some(ref cid) = body.conversation_id {
        if guard.agent.get_conversation(cid).is_some() {
            guard.conv_id = cid.clone();
            cid.clone()
        } else {
            guard
                .agent
                .ensure_conversation_with_tools(
                    &state.model,
                    cid,
                    body.tool_ids.as_deref(),
                )
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            guard.conv_id = cid.clone();
            cid.clone()
        }
    } else {
        guard.conv_id.clone()
    };
    let memory_debug = guard
        .agent
        .preview_memory_debug(&conv_id, body.message.trim(), body.use_memory)
        .await;
    log::info!(
        "HTTP chat memory: used={} source={} items={} use_memory={} conv_id={}",
        memory_debug.memory_used,
        memory_debug.memory_source,
        memory_debug.memory_items_count,
        body.use_memory,
        conv_id
    );
    let tool_ids = body.tool_ids.clone().filter(|v| !v.is_empty());
    let use_tools = body.use_tools;
    let policy = if use_tools {
        Some(kowalski_core::tools::policy::ToolExecutionPolicy {
            allowed_tools: tool_ids,
            sandbox_root: body
                .sandbox_root
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .map(std::path::PathBuf::from),
            quiet: true,
        })
    } else {
        None
    };
    // Honor `use_memory` on the plain path: horde workers send `use_memory: false`.
    let reply = if use_tools {
        guard
            .agent
            .chat_with_tools_with_policy(
                &conv_id,
                body.message.trim(),
                body.use_memory,
                policy.as_ref(),
            )
            .await
    } else {
        guard
            .agent
            .base_mut()
            .chat_with_history_with_options(
                &conv_id,
                body.message.trim(),
                None,
                body.use_memory,
            )
            .await
    }
    .map_err(|e| {
        log::error!("POST /api/chat failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;
    Ok(Json(ChatResponse {
        reply,
        mode: "agent",
        model: state.model.clone(),
        memory_used: memory_debug.memory_used,
        memory_source: memory_debug.memory_source,
        memory_items_count: memory_debug.memory_items_count,
    }))
}

async fn get_chat_messages(
    State(state): State<ApiState>,
    Query(query): Query<ChatMessagesQuery>,
) -> Result<Json<ChatMessagesResponse>, (StatusCode, String)> {
    let guard = state.chat.lock().await;
    let conv_id = query
        .conversation_id
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| guard.conv_id.clone());
    let conversation = guard
        .agent
        .get_conversation(&conv_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "conversation not found".to_string()))?;
    Ok(Json(ChatMessagesResponse {
        conversation_id: conv_id,
        model: state.model.clone(),
        messages: conversation.messages.clone(),
    }))
}

/// SSE (`text/event-stream`): `start`, then `token` deltas, optional final `assistant` echo, then `done`.
/// With `tools_stream: true`, runs the tool loop and emits `token` only for the LLM turn after tool execution(s); with `tools_stream: false` (default), one plain LLM stream (no tool loop).
async fn post_chat_stream(
    State(state): State<ApiState>,
    Json(body): Json<ChatBody>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);
    let msg = body.message.trim().to_string();
    let tools_stream = body.tools_stream;
    let use_memory = body.use_memory;
    let requested_conv_id = body.conversation_id.clone();
    let api = state.clone();
    tokio::spawn(async move {
        let (conv_id, memory_debug) = {
            let mut g = api.chat.lock().await;
            let cid = if let Some(ref requested) = requested_conv_id {
                if g.agent.get_conversation(requested).is_some() {
                    g.conv_id = requested.clone();
                    requested.clone()
                } else {
                    let payload = json!({ "type": "error", "message": format!("conversation not found: {}", requested) });
                    let _ = tx
                        .send(Ok(Event::default().data(payload.to_string())))
                        .await;
                    let _ = tx
                        .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
                        .await;
                    return;
                }
            } else {
                g.conv_id.clone()
            };
            let dbg = g.agent.preview_memory_debug(&cid, &msg, use_memory).await;
            (cid, dbg)
        };
        log::info!(
            "HTTP chat stream memory: used={} source={} items={} use_memory={} conv_id={}",
            memory_debug.memory_used,
            memory_debug.memory_source,
            memory_debug.memory_items_count,
            use_memory,
            conv_id
        );
        let start = json!({
            "type": "start",
            "conversation_id": conv_id,
            "model": api.model,
            "memory_used": memory_debug.memory_used,
            "memory_source": memory_debug.memory_source,
            "memory_items_count": memory_debug.memory_items_count,
        });
        if tx
            .send(Ok(Event::default().data(start.to_string())))
            .await
            .is_err()
        {
            return;
        }

        if tools_stream {
            let (token_tx, mut token_rx) = tokio::sync::mpsc::channel::<String>(256);
            let sse = tx.clone();
            let forward = tokio::spawn(async move {
                while let Some(delta) = token_rx.recv().await {
                    let payload = json!({ "type": "token", "content": delta });
                    if sse
                        .send(Ok(Event::default().data(payload.to_string())))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
            let outcome = {
                let mut guard = api.chat.lock().await;
                guard
                    .agent
                    .chat_with_tools_stream_final_with_options(
                        &conv_id, &msg, &token_tx, use_memory,
                    )
                    .await
            };
            drop(token_tx);
            let _ = forward.await;
            match outcome {
                Ok(full) => {
                    let summary = json!({ "type": "assistant", "content": full });
                    let _ = tx
                        .send(Ok(Event::default().data(summary.to_string())))
                        .await;
                }
                Err(e) => {
                    let payload = json!({ "type": "error", "message": e.to_string() });
                    let _ = tx
                        .send(Ok(Event::default().data(payload.to_string())))
                        .await;
                }
            }
            let _ = tx
                .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
                .await;
            return;
        }

        let prep = {
            let mut guard = api.chat.lock().await;
            guard
                .agent
                .prepare_stream_turn_with_options(&conv_id, &msg, use_memory)
                .await
        };
        let (model, messages, llm) = match prep {
            Ok(x) => x,
            Err(e) => {
                let payload = json!({ "type": "error", "message": e.to_string() });
                let _ = tx
                    .send(Ok(Event::default().data(payload.to_string())))
                    .await;
                let _ = tx
                    .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
                    .await;
                return;
            }
        };
        let mut full = String::new();
        let mut stream = llm.chat_stream(&model, messages);
        while let Some(item) = stream.next().await {
            match item {
                Ok(delta) => {
                    if !delta.is_empty() {
                        full.push_str(&delta);
                        let payload = json!({ "type": "token", "content": delta });
                        if tx
                            .send(Ok(Event::default().data(payload.to_string())))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(e) => {
                    let payload = json!({ "type": "error", "message": e.to_string() });
                    let _ = tx
                        .send(Ok(Event::default().data(payload.to_string())))
                        .await;
                    let _ = tx
                        .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
                        .await;
                    return;
                }
            }
        }
        {
            let mut guard = api.chat.lock().await;
            guard.agent.add_message(&conv_id, "assistant", &full).await;
        }
        let summary = json!({ "type": "assistant", "content": full });
        let _ = tx
            .send(Ok(Event::default().data(summary.to_string())))
            .await;
        let _ = tx
            .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
            .await;
    });
    Sse::new(ReceiverStream::new(rx))
}

#[derive(Deserialize)]
struct FederationStreamQuery {
    topic: Option<String>,
}

async fn get_federation_registry(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let agents = state.federation.registry.list();
    #[cfg(feature = "postgres")]
    if let Some(ref url) = state.full_config.memory.database_url
        && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
        && let Ok(states) = kowalski_core::load_agent_states(url).await
    {
        let merged: Vec<serde_json::Value> = agents
            .iter()
            .map(|a| {
                let mut row = json!({
                    "id": &a.id,
                    "capabilities": &a.capabilities,
                });
                if let (Some(obj), Some(s)) = (row.as_object_mut(), states.get(&a.id)) {
                    obj.insert(
                        "state".into(),
                        serde_json::to_value(s).unwrap_or_else(|_| json!({})),
                    );
                }
                row
            })
            .collect();
        return Json(json!({ "agents": merged }));
    }
    Json(json!({ "agents": agents }))
}

#[derive(Clone, Serialize)]
struct WorkerProfile {
    id: String,
    horde_id: String,
    horde_name: String,
    step: String,
    name: String,
    description: String,
    capability: String,
    agent_id: String,
    command: String,
    args: Vec<String>,
    cwd: String,
    log_dir: String,
}

#[derive(Deserialize)]
struct WorkerControlBody {
    profile_id: String,
}

fn repo_root_from_state(state: &ApiState) -> PathBuf {
    if let Ok(cwd) = std::env::current_dir()
        && cwd.join("Cargo.toml").exists()
        && cwd.join("kowalski-cli").exists()
    {
        return cwd;
    }
    let mut p = state.config_path.clone();
    while let Some(parent) = p.parent() {
        if parent.join("Cargo.toml").exists() && parent.join("kowalski-cli").exists() {
            return parent.to_path_buf();
        }
        p = parent.to_path_buf();
    }
    PathBuf::from("/opt/ml/kowalski")
}

fn worker_profiles(state: &ApiState) -> Vec<WorkerProfile> {
    let root = repo_root_from_state(state);
    let mut out = Vec::new();
    for spec in state.horde_manager.specs.iter() {
        for sub in &spec.sub_agents {
            // Kinds with an in-process step handler need no federation worker.
            if state.horde_manager.step_handlers.contains(&sub.kind) {
                continue;
            }
            let id = format!("{}::{}", spec.id, sub.name);
            out.push(WorkerProfile {
                id: id.clone(),
                horde_id: spec.id.clone(),
                horde_name: spec.display_name.clone(),
                step: sub.name.clone(),
                name: sub.display_name.clone(),
                description: sub.description.clone(),
                capability: sub.capability.clone(),
                agent_id: sub.default_agent_id.clone(),
                command: "cargo".into(),
                args: vec![
                    "run".into(),
                    "-p".into(),
                    "kowalski-cli".into(),
                    "--".into(),
                    "agent-app".into(),
                    "worker".into(),
                    "--role".into(),
                    sub.kind.clone(),
                    "--capability".into(),
                    sub.capability.clone(),
                    "--path".into(),
                    spec.root_path.display().to_string(),
                    sub.default_agent_id.clone(),
                ],
                cwd: root.display().to_string(),
                log_dir: spec.worker_log_dir.display().to_string(),
            });
        }
    }
    out
}

/// Single place defining what a spawned worker inherits to reach this server:
/// its base URL (`KOWALSKI_API`) and the bearer token (`KOWALSKI_API_TOKEN`).
fn export_worker_env(cmd: &mut tokio::process::Command, state: &ApiState) {
    cmd.env(kowalski_core::config::API_URL_ENV, &state.api_url);
    if let Some(token) = state.api_token.as_ref() {
        cmd.env(crate::auth::TOKEN_ENV, token.as_str());
    }
}

fn worker_log_stdio(
    log_dir: &str,
    profile_id: &str,
) -> Option<(std::process::Stdio, std::process::Stdio)> {
    let dir = PathBuf::from(log_dir);
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    let log_name = profile_id.replace("::", "--");
    let path = dir.join(format!("{}.log", log_name));
    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()?;
    let stderr_file = stdout_file.try_clone().ok()?;
    Some((
        std::process::Stdio::from(stdout_file),
        std::process::Stdio::from(stderr_file),
    ))
}

async fn get_federation_workers(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let profiles = worker_profiles(&state);
    let registry = state.federation.registry.list();
    let mut managed = state.managed_workers.lock().await;
    let mut last_exit = state.managed_worker_last_exit.lock().await;

    managed.retain(|profile_id, child| match child.try_wait() {
        Ok(Some(status)) => {
            last_exit.insert(
                profile_id.clone(),
                format!(
                    "exited: code={:?} success={}",
                    status.code(),
                    status.success()
                ),
            );
            false
        }
        Ok(None) => true,
        Err(e) => {
            last_exit.insert(profile_id.clone(), format!("wait error: {}", e));
            false
        }
    });

    let rows: Vec<serde_json::Value> = profiles
        .iter()
        .map(|p| worker_row(p, &managed, &last_exit, &registry))
        .collect();

    Json(json!({ "profiles": rows }))
}

fn worker_row(
    p: &WorkerProfile,
    managed: &HashMap<String, Child>,
    last_exit: &HashMap<String, String>,
    registry: &[kowalski_core::federation::AgentRecord],
) -> serde_json::Value {
    let pid = managed.get(&p.id).and_then(|c| c.id());
    let registered_exact = registry.iter().any(|a| a.id == p.agent_id);
    let registry_ids: Vec<String> = registry
        .iter()
        .filter(|a| a.capabilities.iter().any(|c| c == &p.capability))
        .map(|a| a.id.clone())
        .collect();
    json!({
        "id": p.id,
        "horde_id": p.horde_id,
        "horde_name": p.horde_name,
        "step": p.step,
        "name": p.name,
        "description": p.description,
        "capability": p.capability,
        "agent_id": p.agent_id,
        "command": p.command,
        "args": p.args,
        "cwd": p.cwd,
        "log_dir": p.log_dir,
        "managed_running": pid.is_some(),
        "pid": pid,
        "last_exit": last_exit.get(&p.id).cloned(),
        "registered_exact": registered_exact,
        "stale_registration": registered_exact && pid.is_none(),
        "registry_agents": registry_ids,
    })
}

async fn post_federation_worker_start(
    State(state): State<ApiState>,
    Json(body): Json<WorkerControlBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let profile = worker_profiles(&state)
        .into_iter()
        .find(|p| p.id == body.profile_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown worker profile: {}", body.profile_id),
            )
        })?;

    let mut managed = state.managed_workers.lock().await;
    let mut last_exit = state.managed_worker_last_exit.lock().await;
    if let Some(existing) = managed.get_mut(&profile.id) {
        match existing.try_wait() {
            Ok(Some(_)) | Err(_) => {
                managed.remove(&profile.id);
            }
            Ok(None) => {
                return Ok(Json(json!({
                    "ok": true,
                    "already_running": true,
                    "profile_id": profile.id,
                    "pid": existing.id(),
                })));
            }
        }
    }

    // If registry still contains a stale agent id for this profile, remove it first.
    if state
        .federation
        .registry
        .deregister(&profile.agent_id)
        .is_ok()
    {
        log::info!(
            "federation worker start: removed stale registry agent_id={}",
            profile.agent_id
        );
    }

    let mut cmd = tokio::process::Command::new(&profile.command);
    cmd.args(profile.args.iter()).current_dir(&profile.cwd);
    export_worker_env(&mut cmd, &state);
    if let Some((out, err)) = worker_log_stdio(&profile.log_dir, &profile.id) {
        cmd.stdout(out).stderr(err);
    } else {
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
    }
    let child = cmd.spawn().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("spawn failed: {}", e),
        )
    })?;
    log::info!(
        "federation worker start profile={} agent_id={} pid={:?}",
        profile.id,
        profile.agent_id,
        child.id()
    );
    let pid = child.id();
    managed.insert(profile.id.clone(), child);
    last_exit.remove(&profile.id);

    Ok(Json(json!({
        "ok": true,
        "already_running": false,
        "profile_id": profile.id,
        "pid": pid,
        "command": profile.command,
        "args": profile.args,
        "cwd": profile.cwd,
    })))
}

async fn post_federation_worker_stop(
    State(state): State<ApiState>,
    Json(body): Json<WorkerControlBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let profile = worker_profiles(&state)
        .into_iter()
        .find(|p| p.id == body.profile_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown worker profile: {}", body.profile_id),
            )
        })?;
    let mut managed = state.managed_workers.lock().await;
    let mut last_exit = state.managed_worker_last_exit.lock().await;
    let mut child = managed.remove(&body.profile_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("worker not running: {}", body.profile_id),
        )
    })?;
    let pid = child.id();
    child.kill().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("stop failed: {}", e),
        )
    })?;
    last_exit.insert(profile.id.clone(), "killed by management".to_string());
    let _ = state.federation.registry.deregister(&profile.agent_id);
    log::info!(
        "federation worker stop profile={} agent_id={} pid={:?}",
        profile.id,
        profile.agent_id,
        pid
    );
    Ok(Json(json!({
        "ok": true,
        "profile_id": body.profile_id,
        "pid": pid,
        "deregistered_agent_id": profile.agent_id
    })))
}

#[derive(Deserialize)]
struct FederationHeartbeatBody {
    agent_id: String,
}

async fn post_federation_heartbeat(
    State(state): State<ApiState>,
    Json(body): Json<FederationHeartbeatBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = body.agent_id.trim();
    if id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "agent_id required".into()));
    }
    #[cfg(feature = "postgres")]
    {
        if let Some(ref url) = state.full_config.memory.database_url
            && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
        {
            return kowalski_core::touch_agent_heartbeat(url, id)
                .await
                .map(|_| Json(json!({ "ok": true, "agent_id": id })))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
    // Heartbeat persistence is optional. When Postgres is unavailable, keep worker runtime healthy
    // and report a non-fatal volatile heartbeat status instead of returning 503.
    Ok(Json(json!({
        "ok": true,
        "agent_id": id,
        "persisted": false,
        "mode": "volatile",
        "note": format!(
            "Postgres heartbeat persistence disabled (config {}; run with --features postgres and database_url to persist).",
            state.config_path.display()
        )
    })))
}

async fn get_federation_ws(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
    Query(q): Query<FederationStreamQuery>,
) -> impl IntoResponse {
    let topic = q.topic.unwrap_or_else(|| "federation".to_string());
    ws.on_upgrade(move |socket| federation_ws_task(socket, state, topic))
}

async fn federation_ws_task(mut socket: WebSocket, state: ApiState, topic: String) {
    let mut rx = state.federation_broker.subscribe(&topic, 64);
    loop {
        tokio::select! {
            m = rx.recv() => {
                let Some(env) = m else { break };
                let text = serde_json::to_string(&env).unwrap_or_else(|_| "{}".to_string());
                if socket
                    .send(axum::extract::ws::Message::text(text))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

async fn get_graph_status(State(state): State<ApiState>) -> Json<serde_json::Value> {
    #[cfg(feature = "postgres")]
    {
        if let Some(ref url) = state.full_config.memory.database_url
            && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
        {
            return match kowalski_core::postgres_graph_status(url).await {
                Ok(v) => Json(v),
                Err(e) => Json(json!({ "error": e.to_string() })),
            };
        }
    }
    Json(json!({
        "postgres": false,
        "vector_extension": false,
        "age_extension": false,
        "config_path": state.config_path.display().to_string(),
        "note": "Configure memory.database_url and build with --features postgres for live extension probes."
    }))
}

#[cfg(feature = "postgres")]
#[derive(Deserialize)]
struct GraphCypherBody {
    graph: String,
    query: String,
}

#[cfg(feature = "postgres")]
async fn post_graph_cypher(
    State(state): State<ApiState>,
    Json(body): Json<GraphCypherBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Some(ref url) = state.full_config.memory.database_url
        && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
    {
        return kowalski_core::postgres_age_cypher(url, body.graph.trim(), body.query.trim())
            .await
            .map(Json)
            .map_err(|e| {
                let msg = e.to_string();
                let code = if msg.contains("AGE extension") {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::BAD_REQUEST
                };
                (code, msg)
            });
    }
    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        "Postgres memory URL not configured".to_string(),
    ))
}

/// SSE: one JSON [`AclEnvelope`] per `data:` line (same topic as in-process broker).
async fn get_federation_stream(
    State(state): State<ApiState>,
    Query(q): Query<FederationStreamQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>> + Send> {
    let topic = q.topic.unwrap_or_else(|| "federation".to_string());
    let rx = state.federation_broker.subscribe(&topic, 64);
    let stream = ReceiverStream::new(rx).map(|env| {
        Ok::<Event, Infallible>(
            Event::default().data(serde_json::to_string(&env).unwrap_or_else(|_| "{}".to_string())),
        )
    });
    Sse::new(stream)
}

#[derive(Deserialize)]
struct FederationDelegateBody {
    task_id: String,
    instruction: String,
    capability: String,
}

async fn post_federation_delegate(
    State(state): State<ApiState>,
    Json(body): Json<FederationDelegateBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let outcome = state
        .federation
        .delegate_first_match(&body.task_id, &body.instruction, &body.capability)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    #[cfg(feature = "postgres")]
    if let (Some(url), Some(o)) = (
        state.full_config.memory.database_url.as_ref(),
        outcome.as_ref(),
    ) && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
    {
        let task_label = format!(
            "{}: {}",
            body.task_id,
            body.instruction.chars().take(240).collect::<String>()
        );
        if let Err(e) = kowalski_core::set_agent_current_task(url, &o.agent_id, &task_label).await {
            log::warn!("federation current_task: {}", e);
        }
    }

    #[cfg(feature = "postgres")]
    if let (Some(pg), Some(o)) = (&state.federation_pg_notify, outcome.as_ref())
        && let Err(e) = pg.publish(&o.envelope).await
    {
        log::warn!("federation pg_notify fan-out: {}", e);
    }

    Ok(Json(json!({
        "delegated_to": outcome.as_ref().map(|o| &o.agent_id),
        "topic": outcome.as_ref().map(|o| &o.envelope.topic),
    })))
}

#[derive(Deserialize)]
struct FederationPublishBody {
    sender: String,
    payload: AclMessage,
    topic: Option<String>,
}

async fn post_federation_publish(
    State(state): State<ApiState>,
    Json(body): Json<FederationPublishBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let sender = body.sender.trim();
    if sender.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "sender required".into()));
    }
    let topic = body.topic.unwrap_or_else(|| "federation".to_string());
    let env = AclEnvelope::new(topic, sender.to_string(), body.payload);
    state
        .federation
        .publish(&env)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    #[cfg(feature = "postgres")]
    if let Some(pg) = &state.federation_pg_notify
        && let Err(e) = pg.publish(&env).await
    {
        log::warn!("federation pg_notify fan-out (publish): {}", e);
    }

    Ok(Json(json!({
        "ok": true,
        "id": env.id,
        "topic": env.topic,
        "sender": env.sender,
    })))
}

#[derive(Deserialize)]
struct FederationRegisterBody {
    id: String,
    capabilities: Vec<String>,
}

async fn post_federation_register(
    State(state): State<ApiState>,
    Json(body): Json<FederationRegisterBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = body.id.trim();
    if id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "id required".into()));
    }
    let record = AgentRecord {
        id: id.to_string(),
        capabilities: body.capabilities,
    };
    state
        .federation
        .registry
        .register(record.clone())
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
    #[cfg(feature = "postgres")]
    if let Some(ref url) = state.full_config.memory.database_url
        && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
    {
        if let Err(e) = kowalski_core::upsert_registry_record(url, &record).await {
            log::warn!("federation registry upsert: {}", e);
        }
        if let Err(e) = kowalski_core::upsert_agent_state_for_record(url, &record).await {
            log::warn!("agent_state upsert: {}", e);
        }
    }
    Ok(Json(json!({ "ok": true, "id": record.id })))
}

#[derive(Deserialize)]
struct FederationDeregisterBody {
    agent_id: String,
}

async fn post_federation_deregister(
    State(state): State<ApiState>,
    Json(body): Json<FederationDeregisterBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = body.agent_id.trim();
    if id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "agent_id required".into()));
    }
    if id == "template" {
        return Err((
            StatusCode::FORBIDDEN,
            "cannot deregister built-in template agent".into(),
        ));
    }
    state
        .federation
        .registry
        .deregister(id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    #[cfg(feature = "postgres")]
    if let Some(ref url) = state.full_config.memory.database_url
        && kowalski_core::config::memory_uses_postgres(&state.full_config.memory)
        && let Err(e) = kowalski_core::delete_federation_agent(url, id).await
    {
        log::warn!("federation deregister DB: {}", e);
    }
    Ok(Json(json!({ "ok": true, "agent_id": id })))
}

#[derive(Deserialize)]
struct FederationCleanupBody {
    /// Heartbeats older than this many seconds are treated as stale (`active = false`).
    #[serde(rename = "stale_after_secs")]
    _stale_after_secs: u64,
}

fn state_config_dir(config_path: &std::path::Path) -> Option<PathBuf> {
    config_path.parent().map(|p| p.to_path_buf())
}

#[derive(Deserialize)]
struct HordeRunBody {
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    question: Option<String>,
    /// Raw operator answers keyed by `run_form` field id. The server validates them against the
    /// horde's `run_form` and builds the operator-input block (thin UI / thick core) — the UI does
    /// not pre-render the block or enforce field rules.
    #[serde(default)]
    form_answers: Option<std::collections::BTreeMap<String, String>>,
    /// How the run was started; defaults to `operator` (UI or manual API call).
    /// Non-operator origins (e.g. `trigger`) are auto-resumed after a restart.
    #[serde(default)]
    origin: Option<String>,
}

#[derive(Deserialize)]
struct HordeFollowupBody {
    run_id: String,
    message: String,
}

async fn get_hordes(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let global_clean_on_startup = global_horde_clean_on_startup(&state.full_config);
    let hordes: Vec<serde_json::Value> = state
        .horde_manager
        .specs
        .iter()
        .map(|s| {
            let effective_clean_on_startup = global_clean_on_startup.unwrap_or(s.config_on_startup);
            json!({
                "id": s.id,
                "display_name": s.display_name,
                "description": s.description,
                "capability_prefix": s.capability_prefix,
                "pipeline": s.pipeline,
                "edges": s.manifest_edges,
                "default_question": s.default_question,
                "topic": s.topic,
                "root_path": s.root_path.display().to_string(),
                "workdir": s.workdir.display().to_string(),
                "config_on_startup": s.config_on_startup,
                "config_on_startup_effective": effective_clean_on_startup,
                "delivery_title": s.delivery_title,
                "delivery_note": s.delivery_note,
                "delivery_root_rel": s.delivery_root_rel,
                "delivery_summary_note": s.delivery_summary_note,
                "prompt_tip": s.prompt_tip,
                "sub_agents": s.sub_agents,
                "run_form": s.run_form,
            })
        })
        .collect();
    Json(json!({ "hordes": hordes }))
}

async fn get_horde_detail(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let global_clean_on_startup = global_horde_clean_on_startup(&state.full_config);
    let spec = state.horde_manager.find(&horde_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("unknown horde id: {}", horde_id),
        )
    })?;
    Ok(Json(json!({
        "id": spec.id,
        "display_name": spec.display_name,
        "description": spec.description,
        "capability_prefix": spec.capability_prefix,
        "pipeline": spec.pipeline,
        "edges": spec.manifest_edges,
        "default_question": spec.default_question,
        "topic": spec.topic,
        "root_path": spec.root_path.display().to_string(),
        "workdir": spec.workdir.display().to_string(),
        "config_on_startup": spec.config_on_startup,
        "config_on_startup_effective": global_clean_on_startup.unwrap_or(spec.config_on_startup),
        "delivery_title": spec.delivery_title,
        "delivery_note": spec.delivery_note,
        "delivery_root_rel": spec.delivery_root_rel,
        "delivery_summary_note": spec.delivery_summary_note,
        "prompt_tip": spec.prompt_tip,
        "sub_agents": spec.sub_agents,
        "run_form": spec.run_form,
    })))
}

async fn get_horde_workers(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _spec = state.horde_manager.find(&horde_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("unknown horde id: {}", horde_id),
        )
    })?;
    let mut managed = state.managed_workers.lock().await;
    let mut last_exit = state.managed_worker_last_exit.lock().await;
    managed.retain(|profile_id, child| match child.try_wait() {
        Ok(Some(status)) => {
            last_exit.insert(
                profile_id.clone(),
                format!(
                    "exited: code={:?} success={}",
                    status.code(),
                    status.success()
                ),
            );
            log::info!(
                "horde worker exited profile={} status_code={:?} success={}",
                profile_id,
                status.code(),
                status.success()
            );
            false
        }
        Ok(None) => true,
        Err(e) => {
            last_exit.insert(profile_id.clone(), format!("wait error: {}", e));
            log::warn!("horde worker wait error profile={} error={}", profile_id, e);
            false
        }
    });
    let profiles: Vec<WorkerProfile> = worker_profiles(&state)
        .into_iter()
        .filter(|p| p.horde_id == horde_id)
        .collect();
    // Auto-prune stale registry entries for workers that are no longer managed/running.
    for p in &profiles {
        let pid = managed.get(&p.id).and_then(|c| c.id());
        let is_registered = state
            .federation
            .registry
            .list()
            .iter()
            .any(|a| a.id == p.agent_id);
        let has_exit = last_exit.get(&p.id).is_some();
        if pid.is_none()
            && is_registered
            && has_exit
            && state.federation.registry.deregister(&p.agent_id).is_ok()
        {
            log::info!(
                "horde worker cleanup: deregistered stale agent_id={} profile={}",
                p.agent_id,
                p.id
            );
        }
    }
    let registry = state.federation.registry.list();
    let rows: Vec<serde_json::Value> = profiles
        .into_iter()
        .map(|p| worker_row(&p, &managed, &last_exit, &registry))
        .collect();
    Ok(Json(json!({
        "horde_id": horde_id,
        "workers": rows,
    })))
}

#[derive(Deserialize)]
struct HordeWorkerControlBody {
    /// When provided, only manage this sub-agent's worker. When omitted, target all sub-agents.
    #[serde(default)]
    step: Option<String>,
}

async fn post_horde_worker_start(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
    Json(body): Json<HordeWorkerControlBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _spec = state
        .horde_manager
        .find(&horde_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown horde id: {}", horde_id),
            )
        })?
        .clone();
    let profiles: Vec<WorkerProfile> = worker_profiles(&state)
        .into_iter()
        .filter(|p| p.horde_id == horde_id)
        .filter(|p| body.step.as_deref().map(|s| s == p.step).unwrap_or(true))
        .collect();
    if profiles.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!(
                "no sub-agent workers matched (horde={}, step={:?})",
                horde_id, body.step
            ),
        ));
    }

    let mut started: Vec<serde_json::Value> = Vec::new();
    {
        let mut managed = state.managed_workers.lock().await;
        let mut last_exit = state.managed_worker_last_exit.lock().await;
        for profile in profiles {
            if let Some(existing) = managed.get_mut(&profile.id) {
                match existing.try_wait() {
                    Ok(Some(_)) | Err(_) => {
                        managed.remove(&profile.id);
                    }
                    Ok(None) => {
                        started.push(json!({
                            "profile_id": profile.id,
                            "already_running": true,
                            "pid": existing.id(),
                        }));
                        continue;
                    }
                }
            }
            if state
                .federation
                .registry
                .deregister(&profile.agent_id)
                .is_ok()
            {
                log::info!(
                    "horde worker start: removed stale registry entry for agent_id={}",
                    profile.agent_id
                );
            }
            let mut cmd = tokio::process::Command::new(&profile.command);
            cmd.args(profile.args.iter()).current_dir(&profile.cwd);
            export_worker_env(&mut cmd, &state);
            if let Some((out, err)) = worker_log_stdio(&profile.log_dir, &profile.id) {
                cmd.stdout(out).stderr(err);
            } else {
                cmd.stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
            }
            match cmd.spawn() {
                Ok(child) => {
                    let pid = child.id();
                    log::info!(
                        "horde worker start horde={} step={} agent_id={} pid={:?}",
                        profile.horde_id,
                        profile.step,
                        profile.agent_id,
                        pid
                    );
                    managed.insert(profile.id.clone(), child);
                    last_exit.remove(&profile.id);
                    started.push(json!({
                        "profile_id": profile.id,
                        "already_running": false,
                        "pid": pid,
                    }));
                }
                Err(e) => {
                    started.push(json!({
                        "profile_id": profile.id,
                        "error": format!("spawn failed: {}", e),
                    }));
                }
            }
        }
    }
    Ok(Json(json!({ "ok": true, "started": started })))
}

async fn post_horde_worker_stop(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
    Json(body): Json<HordeWorkerControlBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _spec = state
        .horde_manager
        .find(&horde_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown horde id: {}", horde_id),
            )
        })?
        .clone();
    let profiles: Vec<WorkerProfile> = worker_profiles(&state)
        .into_iter()
        .filter(|p| p.horde_id == horde_id)
        .filter(|p| body.step.as_deref().map(|s| s == p.step).unwrap_or(true))
        .collect();
    let mut stopped: Vec<serde_json::Value> = Vec::new();
    {
        let mut managed = state.managed_workers.lock().await;
        let mut last_exit = state.managed_worker_last_exit.lock().await;
        for profile in profiles {
            match managed.remove(&profile.id) {
                Some(mut child) => {
                    let pid = child.id();
                    if let Err(e) = child.kill().await {
                        stopped.push(json!({
                            "profile_id": profile.id,
                            "error": format!("kill failed: {}", e),
                        }));
                        continue;
                    }
                    last_exit.insert(profile.id.clone(), "killed by management".to_string());
                    let _ = state.federation.registry.deregister(&profile.agent_id);
                    stopped.push(json!({
                        "profile_id": profile.id,
                        "pid": pid,
                        "deregistered_agent_id": profile.agent_id,
                    }));
                }
                None => {
                    let _ = state.federation.registry.deregister(&profile.agent_id);
                    stopped.push(json!({
                        "profile_id": profile.id,
                        "skipped": "not running",
                        "deregistered_agent_id": profile.agent_id,
                    }));
                }
            }
        }
    }
    Ok(Json(json!({ "ok": true, "stopped": stopped })))
}

async fn post_horde_clean_workdir(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let spec = state.horde_manager.find(&horde_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("unknown horde id: {}", horde_id),
        )
    })?;
    crate::horde::clean_horde_workdir(spec).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("clean workdir: {}", e),
        )
    })?;
    log::info!(
        "horde workdir cleaned via API horde={} workdir={}",
        horde_id,
        spec.workdir.display()
    );
    Ok(Json(json!({
        "ok": true,
        "horde_id": horde_id,
        "workdir": spec.workdir.display().to_string(),
    })))
}

async fn post_horde_repair_outputs(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let spec = state.horde_manager.find(&horde_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("unknown horde id: {}", horde_id),
        )
    })?;
    let fixed = kowalski_core::repair_horde_tree_outputs(&spec.root_path).map_err(|e| {
        (StatusCode::BAD_REQUEST, e.to_string())
    })?;
    Ok(Json(json!({
        "ok": true,
        "horde_id": horde_id,
        "files_fixed": fixed,
    })))
}

async fn post_horde_run(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
    Json(body): Json<HordeRunBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Server owns the operator form: validate answers against the horde's run_form and build the
    // operator-input block via kowalski-core (no client-side prompt assembly or validation).
    let operator_block = match (
        body.form_answers.as_ref(),
        state.horde_manager.find(&horde_id).and_then(|s| s.run_form.clone()),
    ) {
        (Some(answers), Some(form)) => {
            kowalski_core::validate_form_answers(&form, answers)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            Some(kowalski_core::answers_to_prompt(&form, answers))
        }
        _ => None,
    };

    let user_prompt = body.prompt.clone().unwrap_or_default();
    let prompt = match &operator_block {
        Some(block) if !user_prompt.trim().is_empty() => format!("{block}\n\n{user_prompt}"),
        Some(block) => block.clone(),
        None => user_prompt,
    };
    let source_extracted = body
        .source
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            let p = prompt.trim();
            if p.is_empty() {
                None
            } else {
                // Keep run input transport generic: prompt can contain URLs, file paths, or plain text.
                Some(p.to_string())
            }
        });
    let inferred_question = body
        .question
        .clone()
        .filter(|q| !q.trim().is_empty())
        .or_else(|| {
            let p = prompt.trim();
            if p.is_empty() {
                None
            } else {
                // Use the full user prompt as run question when explicit `question` is omitted.
                Some(p.to_string())
            }
        });
    let record = state
        .horde_manager
        .start_run(
            &horde_id,
            &prompt,
            source_extracted.as_deref(),
            inferred_question.as_deref(),
            body.origin
                .as_deref()
                .filter(|o| !o.trim().is_empty())
                .unwrap_or(kowalski_core::db::run_store::RUN_ORIGIN_OPERATOR),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({
        "ok": true,
        "run": record,
    })))
}

async fn post_horde_followup(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
    Json(body): Json<HordeFollowupBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let spec = state
        .horde_manager
        .find(&horde_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("unknown horde id: {}", horde_id),
            )
        })?
        .clone();
    let run = state
        .horde_manager
        .persisted_run(&body.run_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("run {} not found", body.run_id),
            )
        })?;
    if run.horde_id != horde_id {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("run {} belongs to horde {}", body.run_id, run.horde_id),
        ));
    }
    if body.message.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message required".to_string()));
    }

    let mut context = String::new();
    context.push_str(&format!(
        "Horde: {}\nRun ID: {}\nOriginal prompt: {}\n\n",
        spec.display_name, run.run_id, run.prompt
    ));
    context.push_str("Artifacts:\n");
    for s in &run.steps {
        if let Some(a) = &s.artifact {
            context.push_str(&format!("- {}: {}\n", s.step, a));
        }
    }
    let mut included = 0usize;
    for s in &run.steps {
        if let Some(a) = &s.artifact
            && let Ok(raw) = std::fs::read_to_string(a)
        {
            let excerpt: String = raw.chars().take(6000).collect();
            context.push_str(&format!(
                "\n## {} artifact excerpt ({})\n{}\n",
                s.step, a, excerpt
            ));
            included += 1;
            if included >= 3 {
                break;
            }
        }
    }

    let llm_prompt = format!(
        "You are the continuation assistant for a completed multi-agent horde run.\n\
         Continue the conversation naturally using artifact context below.\n\
         Do not perform keyword routing or fixed intent rules; just answer the user's follow-up.\n\
         Keep answer practical and concise, include uncertainty if needed.\n\
         IMPORTANT:\n\
         - Do NOT invent nonexistent files, templates, or paths.\n\
         - Ground suggestions in the provided artifacts only.\n\n\
         {}\n\
         User follow-up: {}\n",
        context,
        body.message.trim()
    );

    let mut guard = state.chat.lock().await;
    let conv_id = guard.conv_id.clone();
    let reply = guard
        .agent
        .chat_with_history(&conv_id, &llm_prompt, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Persist follow-up output under workdir (`debug/followups/` — `horde::FOLLOWUP_ARTIFACT_REL`).
    let follow_dir = &spec.followup_artifact_dir;
    let (output_path, _saved_ok) = if let Err(e) = std::fs::create_dir_all(follow_dir) {
        log::warn!("follow-up artifact mkdir failed: {}", e);
        (None, false)
    } else {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let out_path = follow_dir.join(format!("{}-{}.md", run.run_id, stamp));
        let saved = format!(
            "# Horde Follow-up Response\n\n- Horde: {}\n- Run: {}\n- Follow-up: {}\n\n## Response\n\n{}\n",
            spec.display_name,
            run.run_id,
            body.message.trim(),
            reply
        );
        match std::fs::write(&out_path, saved) {
            Ok(()) => (Some(out_path.display().to_string()), true),
            Err(e) => {
                log::warn!("follow-up artifact write failed: {}", e);
                (None, false)
            }
        }
    };
    Ok(Json(json!({
        "ok": true,
        "horde_id": horde_id,
        "run_id": run.run_id,
        "reply": reply,
        "output_path": output_path,
        "mode": "horde_followup_chat_continuation",
    })))
}

#[derive(Deserialize)]
struct RunListQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    offset: Option<i64>,
    /// `?status=resumable` narrows the page to interrupted / awaiting-input
    /// runs that no live orchestrator task owns.
    #[serde(default)]
    status: Option<String>,
}

async fn get_horde_runs(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
    Query(query): Query<RunListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let mut runs = state
        .horde_manager
        .persisted_runs(&horde_id, limit, offset)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if query.status.as_deref() == Some("resumable") {
        runs.retain(|r| r.resumable);
    }
    Ok(Json(json!({
        "horde_id": horde_id,
        "runs": runs,
        "limit": limit,
        "offset": offset,
    })))
}

#[derive(Deserialize, Default)]
struct RunCancelBody {
    #[serde(default)]
    reason: Option<String>,
}

async fn post_horde_run_cancel(
    State(state): State<ApiState>,
    AxumPath((horde_id, run_id)): AxumPath<(String, String)>,
    body: Option<Json<RunCancelBody>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let snap = state
        .horde_manager
        .persisted_run(&run_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("run {} not found", run_id)))?;
    if snap.horde_id != horde_id {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("run {} belongs to horde {}", run_id, snap.horde_id),
        ));
    }
    let reason = body.and_then(|Json(b)| b.reason);
    let record = state
        .horde_manager
        .cancel_run(&run_id, reason.as_deref())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({
        "ok": true,
        "run": record,
    })))
}

async fn post_horde_run_resume(
    State(state): State<ApiState>,
    AxumPath((horde_id, run_id)): AxumPath<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let snap = state
        .horde_manager
        .persisted_run(&run_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("run {} not found", run_id)))?;
    if snap.horde_id != horde_id {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("run {} belongs to horde {}", run_id, snap.horde_id),
        ));
    }
    let record = state
        .horde_manager
        .resume_run(&run_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({
        "ok": true,
        "run": record,
    })))
}

async fn get_horde_run_detail(
    State(state): State<ApiState>,
    AxumPath((horde_id, run_id)): AxumPath<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let snap = state
        .horde_manager
        .persisted_run(&run_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("run {} not found", run_id)))?;
    if snap.horde_id != horde_id {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("run {} belongs to horde {}", run_id, snap.horde_id),
        ));
    }
    Ok(Json(json!({ "run": snap })))
}

async fn post_federation_cleanup_stale(
    State(_state): State<ApiState>,
    Json(_body): Json<FederationCleanupBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    #[cfg(feature = "postgres")]
    {
        if let Some(ref url) = _state.full_config.memory.database_url
            && kowalski_core::config::memory_uses_postgres(&_state.full_config.memory)
        {
            let n = kowalski_core::mark_stale_agents_inactive(url, _body._stale_after_secs)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Ok(Json(json!({ "ok": true, "rows_updated": n })));
        }
    }
    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        "Postgres memory URL not configured".into(),
    ))
}
