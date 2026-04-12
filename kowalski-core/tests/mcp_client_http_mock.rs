//! Integration test: JSON-RPC MCP over HTTP POST against a local mock server
//! (JSON bodies and Streamable HTTP **SSE** bodies).

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::{Json, Router, routing::post};
use kowalski_core::config::{McpServerConfig, McpTransport};
use kowalski_core::mcp::client::McpClient;
use serde_json::{Value, json};
use std::collections::HashMap;

async fn mcp_handler(Json(body): Json<Value>) -> Json<Value> {
    Json(mcp_jsonrpc_result(body))
}

/// Streamable HTTP: same JSON-RPC payloads delivered as `text/event-stream` (`data:` lines).
async fn mcp_handler_sse(Json(body): Json<Value>) -> Response {
    let envelope = mcp_jsonrpc_result(body);
    let data = format!(
        "data: {}\n\n",
        serde_json::to_string(&envelope).expect("serialize json-rpc")
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(data))
        .expect("response")
}

fn mcp_jsonrpc_result(body: Value) -> Value {
    let id = body.get("id").cloned().unwrap_or(json!(1));
    let method = body["method"].as_str().unwrap_or("");

    // JSON-RPC notification (no id): MCP lifecycle after initialize
    if body.get("id").is_none() && method == "notifications/initialized" {
        return json!({ "jsonrpc": "2.0", "result": null });
    }

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name": "mock", "version": "0.1.0"},
            "capabilities": {}
        }),
        "tools/list" => json!({
            "tools": [{
                "name": "echo",
                "description": "Echo input",
                "inputSchema": {
                    "type": "object",
                    "properties": {"msg": {"type": "string", "description": "message"}},
                    "required": []
                }
            }]
        }),
        "tools/call" => json!({
            "content": [{"type": "text", "text": "pong"}]
        }),
        _ => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "method not found"}
            });
        }
    };
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

async fn mcp_handler_with_auth(
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let ok = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        == Some("Bearer test-token");
    if !ok {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(mcp_jsonrpc_result(body)))
}

#[tokio::test]
async fn mcp_client_list_and_call_via_mock_http() {
    let app = Router::new().route("/", post(mcp_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://127.0.0.1:{}/", addr.port());
    let client = McpClient::connect("mock", &url).await.expect("connect");

    let tools = client.list_tools().await.expect("list_tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");
    assert!(!tools[0].description.is_empty());

    let raw = client
        .call_tool("echo", &json!({ "msg": "hi" }))
        .await
        .expect("call_tool");
    let normalized = raw.normalized_content();
    assert_eq!(normalized, json!("pong"));

    server.abort();
}

#[tokio::test]
async fn mcp_client_list_tools_via_sse_response_body() {
    let app = Router::new().route("/", post(mcp_handler_sse));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://127.0.0.1:{}/", addr.port());
    let client = McpClient::connect("mock-sse", &url).await.expect("connect");

    let tools = client.list_tools().await.expect("list_tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    server.abort();
}

#[tokio::test]
async fn mcp_client_sends_mcp_server_config_headers() {
    let app = Router::new().route("/", post(mcp_handler_with_auth));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://127.0.0.1:{}/", addr.port());
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
    let cfg = McpServerConfig {
        name: "mock".to_string(),
        url: url.clone(),
        transport: McpTransport::Http,
        headers,
    };

    let client = McpClient::connect_server(&cfg)
        .await
        .expect("connect with headers");
    let tools = client.list_tools().await.expect("list_tools");
    assert_eq!(tools.len(), 1);

    server.abort();
}
