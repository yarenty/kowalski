//! JSON HTTP API for the Vue operator UI (CORS-enabled for local dev).
//! `/api/chat` and `/api/chat/stream` use one in-process `TemplateAgent` + configured LLM (`[llm]` +
//! `[ollama].model` — Ollama or OpenAI-compatible API).

use axum::extract::Path as AxumPath;
use axum::extract::Query;
use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::Stream;
use futures::StreamExt;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::federation::{
    AclEnvelope, AclMessage, AgentRecord, AgentRegistry, FederationOrchestrator, MpscBroker,
};
#[cfg(feature = "postgres")]
use kowalski_core::federation::MessageBroker;
use kowalski_core::template::agent::TemplateAgent;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

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
    /// Same DB pool as the LISTEN bridge — used to fan out delegates via `NOTIFY`.
    #[cfg(feature = "postgres")]
    federation_pg_notify: Option<Arc<kowalski_core::PgBroker>>,
}

/// Run until SIGINT / process exit. Binds `addr` and serves under `/api/*`.
/// When `tls` is `Some((cert_pem, key_pem))`, serves HTTPS via rustls (`axum-server`).
pub async fn serve(
    addr: SocketAddr,
    config: Option<String>,
    ollama_url: Option<String>,
    tls: Option<(PathBuf, PathBuf)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = crate::http_ops::mcp_config_path(config.as_deref());
    let full_config = crate::http_ops::load_kowalski_config_for_serve(&config_path)?;
    kowalski_core::db::run_memory_migrations_if_configured(&full_config).await?;

    let mut agent = TemplateAgent::new(full_config.clone()).await?;
    let conv_id = agent.start_conversation(&full_config.ollama.model);
    let model = full_config.ollama.model.clone();

    let federation_broker = Arc::new(MpscBroker::new());
    let federation_registry = Arc::new(AgentRegistry::new());
    #[cfg(feature = "postgres")]
    if let Some(ref url) = full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&full_config.memory) {
            if let Err(e) = kowalski_core::load_registry_into(&federation_registry, url).await {
                log::warn!("federation registry DB load: {}", e);
            }
        }
    }
    let template_agent = AgentRecord {
        id: "template".into(),
        capabilities: vec!["chat".into(), "mcp".into(), "llm".into()],
    };
    federation_registry
        .register(template_agent.clone())
        .map_err(|e| format!("federation registry: {e}"))?;
    #[cfg(feature = "postgres")]
    if let Some(ref url) = full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&full_config.memory) {
            if let Err(e) = kowalski_core::upsert_registry_record(url, &template_agent).await {
                log::warn!("federation registry upsert: {}", e);
            }
            if let Err(e) = kowalski_core::upsert_agent_state_for_record(url, &template_agent).await
            {
                log::warn!("agent_state upsert: {}", e);
            }
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
        if kowalski_core::config::memory_uses_postgres(&full_config.memory) {
            if let Some(ref url) = full_config.memory.database_url {
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
    let horde_manager = crate::horde::HordeManager::new(
        horde_specs,
        federation_broker.clone(),
        federation.clone(),
    );
    crate::horde::spawn_orchestrator_loop(horde_manager.clone());

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
        #[cfg(feature = "postgres")]
        federation_pg_notify,
    };

    let router = Router::new()
        .route("/api/health", get(get_health))
        .route("/api/agents", get(get_agents))
        .route("/api/sessions", get(get_sessions))
        .route("/api/doctor", get(get_doctor))
        .route("/api/mcp/servers", get(get_mcp_servers))
        .route("/api/mcp/ping", post(post_mcp_ping))
        .route("/api/memory/status", get(get_memory_status))
        .route("/api/chat", post(post_chat))
        .route("/api/chat/stream", post(post_chat_stream))
        .route("/api/chat/reset", post(post_chat_reset))
        .route("/api/chat/sync", post(post_chat_sync))
        .route("/api/chat/messages", get(get_chat_messages))
        .route("/api/federation/stream", get(get_federation_stream))
        .route("/api/federation/ws", get(get_federation_ws))
        .route("/api/federation/registry", get(get_federation_registry))
        .route("/api/federation/workers", get(get_federation_workers))
        .route("/api/federation/workers/start", post(post_federation_worker_start))
        .route("/api/federation/workers/stop", post(post_federation_worker_stop))
        .route("/api/hordes", get(get_hordes))
        .route("/api/hordes/{horde_id}", get(get_horde_detail))
        .route("/api/hordes/{horde_id}/workers", get(get_horde_workers))
        .route("/api/hordes/{horde_id}/workers/start", post(post_horde_worker_start))
        .route("/api/hordes/{horde_id}/workers/stop", post(post_horde_worker_stop))
        .route("/api/hordes/{horde_id}/run", post(post_horde_run))
        .route("/api/hordes/{horde_id}/runs", get(get_horde_runs))
        .route("/api/hordes/{horde_id}/runs/{run_id}", get(get_horde_run_detail))
        .route("/api/federation/register", post(post_federation_register))
        .route("/api/federation/deregister", post(post_federation_deregister))
        .route(
            "/api/federation/cleanup-stale",
            post(post_federation_cleanup_stale),
        )
        .route("/api/federation/heartbeat", post(post_federation_heartbeat))
        .route("/api/federation/delegate", post(post_federation_delegate))
        .route("/api/federation/publish", post(post_federation_publish))
        .route("/api/graph/status", get(get_graph_status));
    #[cfg(feature = "postgres")]
    let router = router.route("/api/graph/cypher", post(post_graph_cypher));
    let app = router
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(false))
                .on_response(DefaultOnResponse::new()),
        )
        .layer(CorsLayer::permissive());

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
    let episodic =
        kowalski_core::memory::episodic::EpisodicBuffer::open(&state.full_config.memory, llm_provider)
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
        .post(format!("{}/api/embeddings", ollama_url.trim_end_matches('/')))
        .json(&json!({
            "model": embed_model,
            "prompt": "healthcheck",
        }))
        .send()
        .await;

    let (embeddings_ok, embed_error) = match probe {
        Ok(resp) if resp.status().is_success() => (true, None),
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_else(|_| "unknown error".to_string());
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
        return Err((StatusCode::BAD_REQUEST, "messages must not be empty".to_string()));
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
            return Err((StatusCode::NOT_FOUND, format!("conversation not found: {}", cid)));
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
    let reply = if body.use_tools {
        guard
            .agent
            .chat_with_tools_with_options(&conv_id, body.message.trim(), body.use_memory)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    } else {
        // Plain generation path for deterministic app-level workflows.
        guard
            .agent
            .chat_with_history(&conv_id, body.message.trim(), None)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };
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
                        .send(Ok(Event::default().data(r#"{"type":"done"}"#.to_string())))
                        .await;
                    return;
                }
            } else {
                g.conv_id.clone()
            };
            let dbg = g
                .agent
                .preview_memory_debug(&cid, &msg, use_memory)
                .await;
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
                    .chat_with_tools_stream_final_with_options(&conv_id, &msg, &token_tx, use_memory)
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
                .send(Ok(Event::default().data(r#"{"type":"done"}"#.to_string())))
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
                    .send(Ok(Event::default().data(r#"{"type":"done"}"#.to_string())))
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
                        .send(Ok(Event::default().data(r#"{"type":"done"}"#.to_string())))
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
            .send(Ok(Event::default().data(r#"{"type":"done"}"#.to_string())))
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
    if let Some(ref url) = state.full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
            if let Ok(states) = kowalski_core::load_agent_states(url).await {
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
        }
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
}

#[derive(Deserialize)]
struct WorkerControlBody {
    profile_id: String,
}

fn repo_root_from_state(state: &ApiState) -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("Cargo.toml").exists() && cwd.join("kowalski-cli").exists() {
            return cwd;
        }
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
            });
        }
    }
    out
}

fn worker_log_stdio(cwd: &str, profile_id: &str) -> Option<(std::process::Stdio, std::process::Stdio)> {
    let dir = PathBuf::from(cwd).join("examples/knowledge-compiler/scratch/workers");
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
                format!("exited: code={:?} success={}", status.code(), status.success()),
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
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("unknown worker profile: {}", body.profile_id)))?;

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
    if state.federation.registry.deregister(&profile.agent_id).is_ok() {
        log::info!(
            "federation worker start: removed stale registry agent_id={}",
            profile.agent_id
        );
    }

    let mut cmd = tokio::process::Command::new(&profile.command);
    cmd.args(profile.args.iter()).current_dir(&profile.cwd);
    if let Some((out, err)) = worker_log_stdio(&profile.cwd, &profile.id) {
        cmd.stdout(out).stderr(err);
    } else {
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
    }
    let child = cmd
        .spawn()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("spawn failed: {}", e)))?;
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
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("unknown worker profile: {}", body.profile_id)))?;
    let mut managed = state.managed_workers.lock().await;
    let mut last_exit = state.managed_worker_last_exit.lock().await;
    let mut child = managed
        .remove(&body.profile_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("worker not running: {}", body.profile_id)))?;
    let pid = child.id();
    child
        .kill()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("stop failed: {}", e)))?;
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
        if let Some(ref url) = state.full_config.memory.database_url {
            if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
                return kowalski_core::touch_agent_heartbeat(url, id)
                    .await
                    .map(|_| Json(json!({ "ok": true, "agent_id": id })))
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
            }
        }
    }
    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        format!(
            "Postgres memory URL not configured (config {}; build with --features postgres for heartbeat persistence)",
            state.config_path.display()
        ),
    ))
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
        if let Some(ref url) = state.full_config.memory.database_url {
            if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
                return match kowalski_core::postgres_graph_status(url).await {
                    Ok(v) => Json(v),
                    Err(e) => Json(json!({ "error": e.to_string() })),
                };
            }
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
    if let Some(ref url) = state.full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
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
        Ok::<Event, Infallible>(Event::default().data(
            serde_json::to_string(&env).unwrap_or_else(|_| "{}".to_string()),
        ))
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
    if let (Some(ref url), Some(o)) = (state.full_config.memory.database_url.as_ref(), outcome.as_ref())
    {
        if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
            let task_label = format!(
                "{}: {}",
                body.task_id,
                body.instruction.chars().take(240).collect::<String>()
            );
            if let Err(e) = kowalski_core::set_agent_current_task(url, &o.agent_id, &task_label).await {
                log::warn!("federation current_task: {}", e);
            }
        }
    }

    #[cfg(feature = "postgres")]
    if let (Some(pg), Some(o)) = (&state.federation_pg_notify, outcome.as_ref()) {
        if let Err(e) = pg.publish(&o.envelope).await {
            log::warn!("federation pg_notify fan-out: {}", e);
        }
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
    if let Some(pg) = &state.federation_pg_notify {
        if let Err(e) = pg.publish(&env).await {
            log::warn!("federation pg_notify fan-out (publish): {}", e);
        }
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
    if let Some(ref url) = state.full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
            if let Err(e) = kowalski_core::upsert_registry_record(url, &record).await {
                log::warn!("federation registry upsert: {}", e);
            }
            if let Err(e) = kowalski_core::upsert_agent_state_for_record(url, &record).await {
                log::warn!("agent_state upsert: {}", e);
            }
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
    if let Some(ref url) = state.full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
            if let Err(e) = kowalski_core::delete_federation_agent(url, id).await {
                log::warn!("federation deregister DB: {}", e);
            }
        }
    }
    Ok(Json(json!({ "ok": true, "agent_id": id })))
}

#[derive(Deserialize)]
struct FederationCleanupBody {
    /// Heartbeats older than this many seconds are treated as stale (`active = false`).
    #[serde(rename = "stale_after_secs")]
    _stale_after_secs: u64,
}

fn state_config_dir(config_path: &PathBuf) -> Option<PathBuf> {
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
}

async fn get_hordes(State(state): State<ApiState>) -> Json<serde_json::Value> {
    let hordes: Vec<serde_json::Value> = state
        .horde_manager
        .specs
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "display_name": s.display_name,
                "description": s.description,
                "capability_prefix": s.capability_prefix,
                "pipeline": s.pipeline,
                "default_question": s.default_question,
                "topic": s.topic,
                "root_path": s.root_path.display().to_string(),
                "sub_agents": s.sub_agents,
            })
        })
        .collect();
    Json(json!({ "hordes": hordes }))
}

async fn get_horde_detail(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let spec = state
        .horde_manager
        .find(&horde_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("unknown horde id: {}", horde_id)))?;
    Ok(Json(json!({
        "id": spec.id,
        "display_name": spec.display_name,
        "description": spec.description,
        "capability_prefix": spec.capability_prefix,
        "pipeline": spec.pipeline,
        "default_question": spec.default_question,
        "topic": spec.topic,
        "root_path": spec.root_path.display().to_string(),
        "sub_agents": spec.sub_agents,
    })))
}

async fn get_horde_workers(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let _spec = state
        .horde_manager
        .find(&horde_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("unknown horde id: {}", horde_id)))?;
    let mut managed = state.managed_workers.lock().await;
    let mut last_exit = state.managed_worker_last_exit.lock().await;
    managed.retain(|profile_id, child| match child.try_wait() {
        Ok(Some(status)) => {
            last_exit.insert(
                profile_id.clone(),
                format!("exited: code={:?} success={}", status.code(), status.success()),
            );
            false
        }
        Ok(None) => true,
        Err(e) => {
            last_exit.insert(profile_id.clone(), format!("wait error: {}", e));
            false
        }
    });
    let registry = state.federation.registry.list();
    let rows: Vec<serde_json::Value> = worker_profiles(&state)
        .into_iter()
        .filter(|p| p.horde_id == horde_id)
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
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("unknown horde id: {}", horde_id)))?
        .clone();
    let profiles: Vec<WorkerProfile> = worker_profiles(&state)
        .into_iter()
        .filter(|p| p.horde_id == horde_id)
        .filter(|p| body.step.as_deref().map(|s| s == p.step).unwrap_or(true))
        .collect();
    if profiles.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("no sub-agent workers matched (horde={}, step={:?})", horde_id, body.step),
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
            if state.federation.registry.deregister(&profile.agent_id).is_ok() {
                log::info!(
                    "horde worker start: removed stale registry entry for agent_id={}",
                    profile.agent_id
                );
            }
            let mut cmd = tokio::process::Command::new(&profile.command);
            cmd.args(profile.args.iter()).current_dir(&profile.cwd);
            if let Some((out, err)) = worker_log_stdio(&profile.cwd, &profile.id) {
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
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("unknown horde id: {}", horde_id)))?
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

async fn post_horde_run(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
    Json(body): Json<HordeRunBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let prompt = body.prompt.unwrap_or_default();
    let source_extracted = body
        .source
        .clone()
        .or_else(|| extract_url(&prompt))
        .filter(|s| !s.is_empty());
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
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    Ok(Json(json!({
        "ok": true,
        "run": record,
    })))
}

fn extract_url(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '/' && c != ':' && c != '.' && c != '-' && c != '_' && c != '?' && c != '=' && c != '&' && c != '#' && c != '~' && c != '%');
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Some(trimmed.to_string());
        }
    }
    None
}

async fn get_horde_runs(
    State(state): State<ApiState>,
    AxumPath(horde_id): AxumPath<String>,
) -> Json<serde_json::Value> {
    let runs: Vec<crate::horde::RunRecord> = state
        .horde_manager
        .list_runs()
        .await
        .into_iter()
        .filter(|r| r.horde_id == horde_id)
        .collect();
    Json(json!({ "horde_id": horde_id, "runs": runs }))
}

async fn get_horde_run_detail(
    State(state): State<ApiState>,
    AxumPath((horde_id, run_id)): AxumPath<(String, String)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let snap = state
        .horde_manager
        .snapshot(&run_id)
        .await
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
        if let Some(ref url) = _state.full_config.memory.database_url {
            if kowalski_core::config::memory_uses_postgres(&_state.full_config.memory) {
                let n = kowalski_core::mark_stale_agents_inactive(url, _body._stale_after_secs)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                return Ok(Json(json!({ "ok": true, "rows_updated": n })));
            }
        }
    }
    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        "Postgres memory URL not configured".into(),
    ))
}
