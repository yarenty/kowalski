//! Integration test: `[llm] openai_headers` reach an OpenAI-compatible endpoint on the wire.

use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use kowalski_core::config::Config;
use kowalski_core::conversation::Message;
use kowalski_core::llm::create_llm_provider;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

type CapturedHeaders = Arc<Mutex<Vec<HeaderMap>>>;

async fn openai_chat(
    State(captured): State<CapturedHeaders>,
    headers: HeaderMap,
    Json(_body): Json<Value>,
) -> Json<Value> {
    captured.lock().unwrap().push(headers);
    Json(json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1770000000,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "pong" },
            "finish_reason": "stop"
        }]
    }))
}

#[tokio::test]
async fn configured_headers_are_sent_with_the_key() {
    let captured: CapturedHeaders = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/v1/chat/completions", post(openai_chat))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut config = Config::default();
    config.llm.provider = "openai".to_string();
    config.llm.openai_api_key = Some("test-key".to_string());
    config.llm.openai_api_base = Some(format!("http://{addr}/v1"));
    config.llm.openai_headers.insert("X-Org".into(), "my-team".into());
    let provider = create_llm_provider(&config).expect("provider from config");

    let reply = provider
        .chat("test-model", &[Message::text("user", "ping")])
        .await
        .expect("chat");
    assert_eq!(reply, "pong");

    let requests = captured.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].get("x-org").and_then(|v| v.to_str().ok()), Some("my-team"));
    assert_eq!(
        requests[0].get("authorization").and_then(|v| v.to_str().ok()),
        Some("Bearer test-key")
    );
    server.abort();
}
