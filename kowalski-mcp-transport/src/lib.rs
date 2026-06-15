//! Reusable MCP transports for **in-repo** Kowalski servers (tool *source 1*).
//!
//! A server implements [`McpHandler`] — turn one JSON-RPC request value into an optional
//! reply (`None` for notifications) — and picks a transport:
//!
//! - [`run_stdio`] — newline-delimited JSON-RPC on stdin/stdout (logs must go to stderr).
//! - [`serve_http`] / [`http_router`] — **stateless Streamable HTTP**: every POST is
//!   independent, no `Mcp-Session-Id` is issued or required, so the server is trivially
//!   restartable / horizontally scalable. Responds with `application/json` or, when the
//!   client sends `Accept: text/event-stream`, a one-shot SSE `data:` frame.
//!
//! The same handler drives both transports, so a server can offer stdio **and** HTTP with no
//! duplicated dispatch logic.

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::{HeaderMap, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// `Accept` value a client should send for Streamable HTTP (JSON or SSE responses).
pub const ACCEPT_STREAMABLE: &str = "application/json, text/event-stream";

/// Handles a single JSON-RPC request value.
///
/// Return `Some(reply)` for requests (those with an `id`) and `None` for notifications
/// (no `id`, e.g. `notifications/initialized`) — notifications must not be answered.
pub trait McpHandler: Send + Sync + 'static {
    fn handle(&self, request: Value) -> impl std::future::Future<Output = Option<Value>> + Send;
}

/// JSON-RPC parse-error envelope (`-32700`).
fn parse_error(detail: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": Value::Null,
        "error": { "code": -32700, "message": format!("parse error: {detail}") }
    })
}

/// Run the handler over **stdio** (newline-delimited JSON-RPC). Blocks until stdin closes.
///
/// stdout carries the protocol stream; never write logs/diagnostics to it.
pub async fn run_stdio<H: McpHandler>(handler: Arc<H>) -> std::io::Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(trimmed) {
            Ok(value) => handler.handle(value).await,
            Err(e) => Some(parse_error(e.to_string())),
        };
        if let Some(reply) = reply {
            stdout
                .write_all(serde_json::to_string(&reply)?.as_bytes())
                .await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

/// Build a **stateless** Streamable HTTP router (`POST /`). No session is issued or required.
pub fn http_router<H: McpHandler>(handler: Arc<H>) -> Router {
    Router::new()
        .route("/", post(handle_post::<H>))
        .with_state(handler)
}

/// Serve the handler as stateless Streamable HTTP on `addr` until shutdown.
pub async fn serve_http<H: McpHandler>(
    addr: std::net::SocketAddr,
    handler: Arc<H>,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, http_router(handler)).await
}

/// True when the client advertises SSE (`text/event-stream`) in `Accept`.
pub fn wants_sse(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

async fn handle_post<H: McpHandler>(
    State(handler): State<Arc<H>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response<Body> {
    let value: Value = match serde_json::from_str(String::from_utf8_lossy(&body).trim()) {
        Ok(v) => v,
        Err(e) => return json_or_sse(&headers, parse_error(e.to_string())),
    };

    match handler.handle(value).await {
        Some(reply) => json_or_sse(&headers, reply),
        // Notification (no id) → 202 Accepted, empty body (MCP lifecycle).
        None => StatusCode::ACCEPTED.into_response(),
    }
}

/// Encode an envelope as `application/json` or, if the client wants it, a one-shot SSE frame.
fn json_or_sse(headers: &HeaderMap, envelope: Value) -> Response<Body> {
    let body = match serde_json::to_string(&envelope) {
        Ok(s) => s,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))
                .unwrap();
        }
    };

    if wants_sse(headers) {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/event-stream")
            .body(Body::from(format!("data: {body}\n\n")))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Echoing handler: replies to requests, ignores notifications.
    struct Echo;
    impl McpHandler for Echo {
        async fn handle(&self, request: Value) -> Option<Value> {
            let id = request.get("id")?.clone();
            Some(json!({ "jsonrpc": "2.0", "id": id, "result": { "echo": request["method"] } }))
        }
    }

    async fn spawn() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, http_router(Arc::new(Echo)))
                .await
                .unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        format!("http://127.0.0.1:{}/", addr.port())
    }

    #[tokio::test]
    async fn http_request_gets_json_reply_without_session_header() {
        let url = spawn().await;
        let res = reqwest::Client::new()
            .post(&url)
            .header("Accept", "application/json")
            .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
            .send()
            .await
            .unwrap();
        // Stateless: server must NOT issue a session id.
        assert!(res.headers().get("mcp-session-id").is_none());
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["result"]["echo"], "tools/list");
    }

    #[tokio::test]
    async fn http_notification_gets_202_no_body() {
        let url = spawn().await;
        let res = reqwest::Client::new()
            .post(&url)
            .json(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::ACCEPTED);
        assert!(res.text().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn http_sse_accept_yields_event_stream() {
        let url = spawn().await;
        let res = reqwest::Client::new()
            .post(&url)
            .header("Accept", ACCEPT_STREAMABLE)
            .json(&json!({ "jsonrpc": "2.0", "id": 5, "method": "ping" }))
            .send()
            .await
            .unwrap();
        let ctype = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        assert!(ctype.contains("text/event-stream"));
        let body = res.text().await.unwrap();
        assert!(body.starts_with("data: "));
    }

    #[tokio::test]
    async fn http_bad_json_returns_parse_error() {
        let url = spawn().await;
        let res = reqwest::Client::new()
            .post(&url)
            .header("Accept", "application/json")
            .body("{not json")
            .send()
            .await
            .unwrap();
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["error"]["code"], -32700);
    }
}
