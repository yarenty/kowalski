//! JSON HTTP API for the Vue operator UI (CORS-enabled for local dev).
//! `/api/chat` and `/api/chat/stream` use one in-process `TemplateAgent` + configured LLM (`[llm]` +
//! `[ollama].model` — Ollama or OpenAI-compatible API).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::extract::Query;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::Stream;
use futures::StreamExt;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::federation::{
    AgentRecord, AgentRegistry, FederationOrchestrator, MpscBroker,
};
#[cfg(feature = "postgres")]
use kowalski_core::federation::MessageBroker;
use kowalski_core::template::agent::TemplateAgent;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};

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
    let config_path = crate::ops::mcp_config_path(config.as_deref());
    let full_config = crate::ops::load_kowalski_config_for_serve(&config_path)?;
    kowalski_core::db::run_memory_migrations_if_configured(&full_config).await?;

    let mut agent = TemplateAgent::new(full_config.clone()).await?;
    let conv_id = agent.start_conversation(&full_config.ollama.model);
    let model = full_config.ollama.model.clone();

    let federation_broker = Arc::new(MpscBroker::new());
    let federation_registry = Arc::new(AgentRegistry::new());
    #[cfg(feature = "postgres")]
    if let Some(ref url) = full_config.memory.database_url {
        if kowalski_core::config::memory_uses_postgres(&full_config.memory) {
            if let Err(e) =
                kowalski_core::load_registry_into(&federation_registry, url).await
            {
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
    let mut federation = FederationOrchestrator::new(
        federation_registry.clone(),
        federation_broker.clone(),
    );
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

    let state = ApiState {
        config_path,
        ollama_url,
        model,
        full_config: full_config.clone(),
        chat: Arc::new(Mutex::new(ChatState { agent, conv_id })),
        federation_broker: federation_broker.clone(),
        federation,
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
        .route("/api/chat", post(post_chat))
        .route("/api/chat/stream", post(post_chat_stream))
        .route("/api/chat/reset", post(post_chat_reset))
        .route("/api/federation/stream", get(get_federation_stream))
        .route("/api/federation/ws", get(get_federation_ws))
        .route("/api/federation/registry", get(get_federation_registry))
        .route("/api/federation/register", post(post_federation_register))
        .route("/api/federation/deregister", post(post_federation_deregister))
        .route("/api/federation/cleanup-stale", post(post_federation_cleanup_stale))
        .route("/api/federation/heartbeat", post(post_federation_heartbeat))
        .route("/api/federation/delegate", post(post_federation_delegate))
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
        "service": "kowalski-cli",
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

async fn get_doctor(State(state): State<ApiState>) -> Json<crate::ops::DoctorJson> {
    Json(
        crate::ops::doctor_json(state.ollama_url.clone(), Some(&state.full_config)).await,
    )
}

async fn get_mcp_servers(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::ops::McpServerPublic>>, (StatusCode, String)> {
    crate::ops::list_mcp_servers_public(&state.config_path)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn post_mcp_ping(
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::ops::McpPingResult>>, (StatusCode, String)> {
    crate::ops::mcp_ping_results(&state.config_path)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Deserialize)]
struct ChatBody {
    message: String,
    /// When true, `POST /api/chat/stream` runs the tool loop and streams **only** the first LLM turn after a tool result (final answer); earlier turns are non-streamed like `POST /api/chat`.
    #[serde(default)]
    tools_stream: bool,
}

#[derive(Serialize)]
struct ChatResponse {
    reply: String,
    mode: &'static str,
    model: String,
}

#[derive(Serialize)]
struct ChatResetResponse {
    conversation_id: String,
    model: String,
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

async fn post_chat(
    State(state): State<ApiState>,
    Json(body): Json<ChatBody>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let mut guard = state.chat.lock().await;
    let conv_id = guard.conv_id.clone();
    let reply = guard
        .agent
        .chat_with_tools(&conv_id, body.message.trim())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ChatResponse {
        reply,
        mode: "agent",
        model: state.model.clone(),
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
    let api = state.clone();
    tokio::spawn(async move {
        let conv_id = {
            let g = api.chat.lock().await;
            g.conv_id.clone()
        };
        let start = json!({
            "type": "start",
            "conversation_id": conv_id,
            "model": api.model,
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
                    .chat_with_tools_stream_final(&conv_id, &msg, &token_tx)
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
            guard.agent.prepare_stream_turn(&conv_id, &msg).await
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
            guard
                .agent
                .add_message(&conv_id, "assistant", &full)
                .await;
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
    if let (Some(ref url), Some(o)) = (
        state.full_config.memory.database_url.as_ref(),
        outcome.as_ref(),
    ) {
        if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
            let task_label = format!(
                "{}: {}",
                body.task_id,
                body.instruction.chars().take(240).collect::<String>()
            );
            if let Err(e) = kowalski_core::set_agent_current_task(url, &o.agent_id, &task_label).await
            {
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
    stale_after_secs: u64,
}

async fn post_federation_cleanup_stale(
    State(state): State<ApiState>,
    Json(body): Json<FederationCleanupBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    #[cfg(feature = "postgres")]
    {
        if let Some(ref url) = state.full_config.memory.database_url {
            if kowalski_core::config::memory_uses_postgres(&state.full_config.memory) {
                let n = kowalski_core::mark_stale_agents_inactive(url, body.stale_after_secs)
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
