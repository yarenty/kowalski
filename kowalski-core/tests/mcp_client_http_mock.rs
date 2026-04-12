//! Integration test: JSON-RPC MCP over HTTP POST against a local mock server.

use axum::{Json, Router, routing::post};
use kowalski_core::mcp::client::McpClient;
use serde_json::{Value, json};

async fn mcp_handler(Json(body): Json<Value>) -> Json<Value> {
    let id = body.get("id").cloned().unwrap_or(json!(1));
    let method = body["method"].as_str().unwrap_or("");
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
            return Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "method not found"}
            }));
        }
    };
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
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
