//! OAuth-connected MCP server: an expired or rejected access token is refreshed through the
//! token endpoint (refresh-token rotation saved to the token file) and the call is retried.

use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use kowalski_core::config::{McpOAuthConfig, McpServerConfig, McpTransport};
use kowalski_core::mcp::client::McpClient;
use kowalski_core::mcp::oauth::TokenSet;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

async fn mcp(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    if auth != "Bearer fresh" {
        return (StatusCode::UNAUTHORIZED, "token expired").into_response();
    }
    let id = body.get("id").cloned();
    let method = body["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => json!({ "protocolVersion": "2025-03-26", "serverInfo": {"name": "mock", "version": "1"}, "capabilities": {} }),
        "tools/list" => json!({ "tools": [{ "name": "echo", "description": "Echo", "inputSchema": {"type": "object", "properties": {}} }] }),
        _ => json!({}),
    };
    match id {
        Some(id) => Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response(),
        None => Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn rejected_token_is_refreshed_rotated_and_retried() {
    let refreshes = Arc::new(AtomicUsize::new(0));
    let counter = refreshes.clone();
    let app = Router::new().route("/mcp", post(mcp)).route(
        "/oauth/token",
        post(move |body: String| {
            let counter = counter.clone();
            async move {
                assert!(body.contains("grant_type=refresh_token") && body.contains("refresh_token=r1"), "{body}");
                counter.fetch_add(1, Ordering::SeqCst);
                Json(json!({ "access_token": "fresh", "refresh_token": "r2", "expires_in": 3600, "token_type": "Bearer" }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("oauth/mock.json");
    TokenSet {
        client_id: "kowalski-test".into(),
        client_secret: None,
        token_endpoint: format!("{base}/oauth/token"),
        access_token: "stale".into(),
        refresh_token: Some("r1".into()),
        expires_at: 0, // unknown expiry: only the server's 401 reveals it
    }
    .save(&token_file)
    .unwrap();

    let client = McpClient::connect_server(&McpServerConfig {
        name: "mock".into(),
        url: format!("{base}/mcp"),
        transport: McpTransport::Http,
        headers: HashMap::new(),
        command: Vec::new(),
        oauth: Some(McpOAuthConfig { token_file: token_file.clone() }),
    })
    .await
    .unwrap();
    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools[0].name, "echo");
    assert_eq!(refreshes.load(Ordering::SeqCst), 1, "one refresh, then the fresh token is reused");

    let saved = TokenSet::load(&token_file).unwrap();
    assert_eq!(saved.access_token, "fresh");
    assert_eq!(saved.refresh_token.as_deref(), Some("r2"), "rotated refresh token saved");
    assert!(saved.expires_at > 0);
}
