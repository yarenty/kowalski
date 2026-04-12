//! Minimal JSON HTTP API for the Vue operator UI (CORS-enabled for local dev).

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct ApiState {
    config_path: PathBuf,
    ollama_url: Option<String>,
}

/// Run until SIGINT / process exit. Binds `addr` and serves under `/api/*`.
pub async fn serve(
    addr: SocketAddr,
    config: Option<String>,
    ollama_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = crate::ops::mcp_config_path(config.as_deref());
    log::info!(
        "Kowalski HTTP API at http://{} (config {})",
        addr,
        config_path.display()
    );
    let state = ApiState {
        config_path,
        ollama_url,
    };

    let app = Router::new()
        .route("/api/health", get(get_health))
        .route("/api/doctor", get(get_doctor))
        .route("/api/mcp/servers", get(get_mcp_servers))
        .route("/api/mcp/ping", post(post_mcp_ping))
        .route("/api/chat", post(post_chat))
        .with_state(state)
        .layer(CorsLayer::permissive());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "kowalski-cli",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn get_doctor(State(state): State<ApiState>) -> Json<crate::ops::DoctorJson> {
    Json(crate::ops::doctor_json(state.ollama_url.clone()).await)
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
}

async fn post_chat(Json(body): Json<ChatBody>) -> Json<ChatResponse> {
    Json(ChatResponse {
        reply: format!(
            "(demo) Full agent chat is not wired to this API yet. You sent: {}",
            body.message
        ),
        mode: "echo",
    })
}
