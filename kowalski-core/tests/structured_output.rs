//! Integration test: constrained structured output against a local mock LLM server.
//!
//! Both providers carry the real Rookery delta schema (the KWC consumer of this surface) on
//! the wire — Ollama in `format`, OpenAI-compatible as strict `response_format: json_schema` —
//! and the constrained reply parses straight into a `DeltaBatch`.

use axum::{Json, Router, extract::State, routing::post};
use kowalski_core::conversation::Message;
use kowalski_core::llm::{LLMProvider, OllamaProvider, OpenAIProvider};
use kowalski_core::rookery::{DeltaBatch, DeltaOp, DeltaSchemaOptions, build_delta_schema};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

type CapturedRequests = Arc<Mutex<Vec<Value>>>;

fn delta_batch_reply() -> String {
    json!({ "ops": [ { "op": "add_step", "step_id": "collect", "kind": "ingest" } ] }).to_string()
}

async fn ollama_chat(
    State(captured): State<CapturedRequests>,
    Json(body): Json<Value>,
) -> Json<Value> {
    captured.lock().unwrap().push(body);
    Json(json!({
        "model": "test-model",
        "message": { "role": "assistant", "content": delta_batch_reply() },
        "done": true
    }))
}

async fn openai_chat(
    State(captured): State<CapturedRequests>,
    Json(body): Json<Value>,
) -> Json<Value> {
    captured.lock().unwrap().push(body);
    Json(json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1770000000,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": delta_batch_reply() },
            "finish_reason": "stop"
        }]
    }))
}

fn assert_reply_parses_as_delta_batch(raw: &str) {
    let value: Value = serde_json::from_str(raw).expect("constrained reply is JSON");
    let batch = DeltaBatch::from_value(&value).expect("reply parses as DeltaBatch");
    assert_eq!(batch.ops.len(), 1);
    match &batch.ops[0] {
        DeltaOp::AddStep { step_id, kind, .. } => {
            assert_eq!(step_id, "collect");
            assert_eq!(kind, "ingest");
        }
        other => panic!("expected add_step, got {other:?}"),
    }
}

#[tokio::test]
async fn ollama_carries_schema_in_format_and_reply_parses() {
    let captured: CapturedRequests = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/api/chat", post(ollama_chat))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let schema = build_delta_schema(DeltaSchemaOptions {
        max_ops: 3,
        allow_replace_draft: false,
    });
    let provider =
        OllamaProvider::new(&addr.ip().to_string(), addr.port()).with_structured_output(true);
    assert!(provider.supports_structured_output("test-model"));

    let raw = provider
        .chat_with_schema(
            "test-model",
            &[Message::text("user", "Add a collect step.")],
            &schema,
        )
        .await
        .expect("structured chat");
    assert_reply_parses_as_delta_batch(&raw);

    let requests = captured.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["format"], schema, "schema travels in `format`");
    assert!(requests[0].get("tools").is_some_and(Value::is_null));

    server.abort();
}

#[tokio::test]
async fn openai_carries_strict_json_schema_response_format_and_reply_parses() {
    let captured: CapturedRequests = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/v1/chat/completions", post(openai_chat))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let schema = build_delta_schema(DeltaSchemaOptions {
        max_ops: 3,
        allow_replace_draft: false,
    });
    let provider = OpenAIProvider::new("test-key", Some(&format!("http://{}/v1", addr)))
        .with_structured_output(true);
    assert!(provider.supports_structured_output("test-model"));

    let raw = provider
        .chat_with_schema(
            "test-model",
            &[Message::text("user", "Add a collect step.")],
            &schema,
        )
        .await
        .expect("structured chat");
    assert_reply_parses_as_delta_batch(&raw);

    let requests = captured.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    let rf = &requests[0]["response_format"];
    assert_eq!(rf["type"], "json_schema");
    assert_eq!(rf["json_schema"]["strict"], true);
    assert_eq!(rf["json_schema"]["name"], "DeltaBatch");
    assert_eq!(rf["json_schema"]["schema"], schema);

    server.abort();
}
