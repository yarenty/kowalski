//! JSON HTTP API for the Vue operator UI (CORS-enabled for local dev).
//! `/api/chat` and `/api/chat/stream` use one in-process `TemplateAgent` + configured LLM (`[llm]` +
//! `[ollama].model` — Ollama or OpenAI-compatible API).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::extract::Query;
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
use kowalski_core::template::agent::TemplateAgent;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

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
}

/// Run until SIGINT / process exit. Binds `addr` and serves under `/api/*`.
pub async fn serve(
    addr: SocketAddr,
    config: Option<String>,
    ollama_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = crate::ops::mcp_config_path(config.as_deref());
    let full_config = crate::ops::load_kowalski_config_for_serve(&config_path)?;
    kowalski_core::db::run_memory_migrations_if_configured(&full_config).await?;

    let mut agent = TemplateAgent::new(full_config.clone()).await?;
    let conv_id = agent.start_conversation(&full_config.ollama.model);
    let model = full_config.ollama.model.clone();

    let federation_broker = Arc::new(MpscBroker::new());
    let federation_registry = Arc::new(AgentRegistry::new());
    federation_registry
        .register(AgentRecord {
            id: "template".into(),
            capabilities: vec!["chat".into(), "mcp".into(), "llm".into()],
        })
        .map_err(|e| format!("federation registry: {e}"))?;
    let mut federation = FederationOrchestrator::new(
        federation_registry.clone(),
        federation_broker.clone(),
    );
    federation.orchestrator_id = "kowalski-serve".into();
    federation.default_topic = "federation".into();
    let federation = Arc::new(federation);

    #[cfg(feature = "postgres")]
    {
        if kowalski_core::config::memory_uses_postgres(&full_config.memory) {
            if let Some(ref url) = full_config.memory.database_url {
                match kowalski_core::bridge_postgres_notify_to_mpsc(
                    url,
                    "kowalski_federation",
                    federation_broker.clone(),
                )
                .await
                {
                    Ok(()) => log::info!(
                        "Federation: Postgres LISTEN kowalski_federation → in-process broker (SSE)"
                    ),
                    Err(e) => log::warn!("Federation Postgres bridge: {}", e),
                }
            }
        }
    }

    log::info!(
        "Kowalski HTTP API at http://{} (config {}, model {})",
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
    };

    let app = Router::new()
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
        .route("/api/federation/registry", get(get_federation_registry))
        .route("/api/federation/delegate", post(post_federation_delegate))
        .with_state(state)
        .layer(CorsLayer::permissive());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_health(State(state): State<ApiState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "kowalski-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "model": state.model,
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

/// SSE (`text/event-stream`): one JSON object per `data:` line — `start`, `assistant` or `error`, then `done`.
/// Token streaming is not wired in core yet; the assistant payload is sent once when the turn completes.
async fn post_chat_stream(
    State(state): State<ApiState>,
    Json(body): Json<ChatBody>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(16);
    let msg = body.message.trim().to_string();
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
        let result = {
            let mut guard = api.chat.lock().await;
            guard.agent.chat_with_tools(&conv_id, &msg).await
        };
        match result {
            Ok(reply) => {
                let payload = json!({ "type": "assistant", "content": reply });
                let _ = tx
                    .send(Ok(Event::default().data(payload.to_string())))
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
            .send(Ok(
                Event::default().data(r#"{"type":"done"}"#.to_string()),
            ))
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
    Json(json!({ "agents": agents }))
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
    let delegated_to = state
        .federation
        .delegate_first_match(&body.task_id, &body.instruction, &body.capability)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "delegated_to": delegated_to })))
}
