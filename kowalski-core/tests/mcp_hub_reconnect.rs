//! Integration test: an MCP server that is down when the hub starts is picked up once it runs,
//! without rebuilding the hub (tableski started after kowalski).

use axum::{Json, Router, routing::post};
use kowalski_core::config::{McpServerConfig, McpTransport};
use kowalski_core::mcp::McpHub;
use kowalski_core::tools::ToolInput;
use kowalski_core::tools::manager::ToolManager;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;

async fn mcp_handler(Json(body): Json<Value>) -> Json<Value> {
    let id = body.get("id").cloned().unwrap_or(json!(1));
    let result = match body["method"].as_str().unwrap_or("") {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {"name": "late", "version": "0.1.0"},
            "capabilities": {}
        }),
        "tools/list" => json!({
            "tools": [{
                "name": "list_tables",
                "description": "List tables",
                "inputSchema": {"type": "object", "properties": {}, "required": []}
            }]
        }),
        "tools/call" => json!({ "content": [{"type": "text", "text": "people, orders"}] }),
        _ => json!(null),
    };
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

#[tokio::test]
async fn server_started_after_the_hub_is_picked_up() {
    // Reserve a port, then leave it closed: the server is "not running yet".
    let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);

    let cfg = McpServerConfig {
        name: "tableski".to_string(),
        url: format!("http://{addr}/"),
        transport: McpTransport::Http,
        headers: HashMap::new(),
        command: Vec::new(),
        oauth: None,
    };
    let hub = McpHub::new(&[cfg]).await.unwrap().expect("a hub even with no server reachable");
    assert_eq!(hub.pending_servers().await, vec!["tableski".to_string()]);

    let tools = ToolManager::new();
    hub.attach(&tools);
    assert!(tools.get("list_tables").is_none());
    assert!(tools.get_or_refresh("list_tables").await.is_none(), "still down");

    // Now the server starts on the same address.
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tokio::spawn(async move {
        axum::serve(listener, Router::new().route("/", post(mcp_handler))).await.unwrap();
    });
    // On-demand retries are at most one per second.
    tokio::time::sleep(Duration::from_millis(1100)).await;

    let out = tools
        .execute("list_tables", ToolInput::new("list_tables".into(), String::new(), json!({})))
        .await
        .expect("the tool appears on first use");
    assert!(out.result.to_string().contains("people"), "{:?}", out.result);
    assert!(hub.pending_servers().await.is_empty());
}
