//! HTTP integration: JSON-RPC initialize, tools/list, tools/call against the real router + DataFusion.

use datafusion::prelude::*;
use kowalski_mcp_datafusion::{AppState, app_router};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/sample.csv");

async fn spawn_server() -> (String, tokio::task::JoinHandle<()>) {
    let ctx = SessionContext::new();
    ctx.register_csv("data", FIXTURE, CsvReadOptions::new())
        .await
        .expect("register_csv");
    let state = AppState::new(Arc::new(ctx), "data", "smoke-test-session");
    let app = app_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let url = format!("http://127.0.0.1:{}/", addr.port());
    let serve = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (url, serve)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("client")
}

#[tokio::test]
async fn mcp_initialize_list_and_query_json() {
    let (url, _serve) = spawn_server().await;

    let c = client();
    let init: Value = c
        .post(&url)
        .header("Accept", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": { "tools": {} },
                "clientInfo": { "name": "test", "version": "0" }
            }
        }))
        .send()
        .await
        .expect("init")
        .json()
        .await
        .expect("init json");
    assert!(init.get("result").is_some(), "{:?}", init);

    let list: Value = c
        .post(&url)
        .header("Accept", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .expect("list")
        .json()
        .await
        .expect("list json");
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 3);
    let names: Vec<_> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"query_sql"));
    assert!(names.contains(&"get_schema"));
    assert!(names.contains(&"column_statistics"));

    let call: Value = c
        .post(&url)
        .header("Accept", "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "query_sql",
                "arguments": { "sql": "SELECT name, score FROM data ORDER BY id" }
            }
        }))
        .send()
        .await
        .expect("call")
        .json()
        .await
        .expect("call json");
    let text = call["result"]["content"][0]["text"]
        .as_str()
        .expect("tool text");
    assert!(text.contains("alpha") && text.contains("beta"));
}

#[tokio::test]
async fn mcp_sse_response_has_event_stream_and_data_line() {
    let (url, _serve) = spawn_server().await;

    let res = client()
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .expect("list");

    assert!(
        res.headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("text/event-stream")
    );
    let body = res.text().await.expect("body");
    assert!(body.starts_with("data: "));
    assert!(body.contains("\"tools\""));
}
