//! Integration test: native provider tool calling against a local mock LLM server.
//!
//! Scripted exchange for both providers (Ollama wire format and OpenAI Chat Completions):
//! the first request declares tools and gets structured `tool_calls` back (no text
//! parsing), a real [`Tool`] executes, and the follow-up request carries the result as a
//! `role = "tool"` message.

use axum::{Json, Router, extract::State, routing::post};
use kowalski_core::conversation::{Conversation, Message};
use kowalski_core::llm::{
    ChatOutcome, LLMProvider, OllamaProvider, OpenAIProvider, ToolDefinition,
};
use kowalski_core::tools::{ParameterType, Tool, ToolInput, ToolOutput, ToolParameter};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct WeatherTool;

#[async_trait::async_trait]
impl Tool for WeatherTool {
    async fn execute(
        &mut self,
        input: ToolInput,
    ) -> Result<ToolOutput, kowalski_core::error::KowalskiError> {
        let location = input.parameters["location"].as_str().unwrap_or("unknown");
        Ok(ToolOutput::new(
            json!(format!("12 degrees and cloudy in {}", location)),
            None,
        ))
    }

    fn name(&self) -> &str {
        "get_current_weather"
    }

    fn description(&self) -> &str {
        "Get the current weather for a location"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![ToolParameter {
            name: "location".to_string(),
            description: "The city name".to_string(),
            required: true,
            default_value: None,
            parameter_type: ParameterType::String,
        }]
    }
}

type CapturedRequests = Arc<Mutex<Vec<Value>>>;

fn request_has_tool_message(body: &Value) -> bool {
    body["messages"]
        .as_array()
        .is_some_and(|msgs| msgs.iter().any(|m| m["role"] == "tool"))
}

async fn ollama_chat(
    State(captured): State<CapturedRequests>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let follow_up = request_has_tool_message(&body);
    captured.lock().unwrap().push(body);
    if follow_up {
        Json(json!({
            "model": "test-model",
            "message": { "role": "assistant", "content": "It is 12 degrees and cloudy in Paris." },
            "done": true
        }))
    } else {
        Json(json!({
            "model": "test-model",
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    { "function": { "name": "get_current_weather", "arguments": { "location": "Paris" } } }
                ]
            },
            "done": true
        }))
    }
}

async fn openai_chat(
    State(captured): State<CapturedRequests>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let follow_up = request_has_tool_message(&body);
    captured.lock().unwrap().push(body);
    let message = if follow_up {
        json!({ "role": "assistant", "content": "It is 12 degrees and cloudy in Paris." })
    } else {
        json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": "call_abc123",
                "type": "function",
                "function": { "name": "get_current_weather", "arguments": "{\"location\": \"Paris\"}" }
            }]
        })
    };
    Json(json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1770000000,
        "model": "test-model",
        "choices": [{ "index": 0, "message": message, "finish_reason": if follow_up { "stop" } else { "tool_calls" } }]
    }))
}

/// Runs the scripted two-turn exchange and returns the captured request bodies.
async fn scripted_exchange(provider: &dyn LLMProvider, captured: CapturedRequests) -> Vec<Value> {
    let mut tool = WeatherTool;
    let tools = vec![ToolDefinition::from_tool(&tool)];
    let mut history = vec![Message::text("user", "What is the weather in Paris?")];

    // Turn 1: structured tool calls, no text parsing.
    let outcome = provider
        .chat_with_tool_defs("test-model", &history, &tools)
        .await
        .expect("first turn");
    let calls = match outcome {
        ChatOutcome::ToolCalls { calls, .. } => calls,
        other => panic!("expected structured tool calls, got {:?}", other),
    };
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].function.name, "get_current_weather");
    assert_eq!(calls[0].function.arguments["location"], "Paris");

    // Execute the real tool with the structured arguments.
    let input = ToolInput::new(
        calls[0].function.name.clone(),
        String::new(),
        calls[0].function.arguments.clone(),
    );
    let result = tool.execute(input).await.expect("tool execution");

    // Turn 2: tool result goes back as a role="tool" message.
    history.push(Message::assistant_tool_calls("", calls.clone()));
    history.push(Message::tool_result(
        &calls[0].id,
        result.result.as_str().unwrap(),
    ));
    let outcome = provider
        .chat_with_tool_defs("test-model", &history, &tools)
        .await
        .expect("second turn");
    match outcome {
        ChatOutcome::Text(text) => assert!(text.contains("12 degrees")),
        other => panic!("expected final text, got {:?}", other),
    }

    let captured = captured.lock().unwrap();
    captured.clone()
}

fn assert_scripted_wire(requests: &[Value]) {
    assert_eq!(requests.len(), 2);
    // First request declares the tool.
    let tools = requests[0]["tools"].as_array().expect("tools on the wire");
    assert_eq!(tools[0]["function"]["name"], "get_current_weather");
    // Follow-up request carries the tool-role result.
    let messages = requests[1]["messages"].as_array().unwrap();
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool-role message in follow-up");
    assert!(
        tool_msg["content"]
            .as_str()
            .unwrap()
            .contains("12 degrees and cloudy in Paris")
    );
}

#[tokio::test]
async fn ollama_scripted_tool_exchange() {
    let captured: CapturedRequests = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/api/chat", post(ollama_chat))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let provider = OllamaProvider::new(&addr.ip().to_string(), addr.port()).with_native_tools(true);
    let requests = scripted_exchange(&provider, captured).await;
    assert_scripted_wire(&requests);
    // Ollama request pairs the result positionally; the id survives for uniformity.
    let messages = requests[1]["messages"].as_array().unwrap();
    let tool_msg = messages.iter().find(|m| m["role"] == "tool").unwrap();
    assert_eq!(tool_msg["tool_call_id"], "call_0");

    server.abort();
}

#[tokio::test]
async fn openai_scripted_tool_exchange() {
    let captured: CapturedRequests = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/v1/chat/completions", post(openai_chat))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let base = format!("http://{}/v1", addr);
    let provider = OpenAIProvider::new("test-key", Some(&base)).with_native_tools(true);
    let requests = scripted_exchange(&provider, captured).await;
    assert_scripted_wire(&requests);
    // Chat Completions requires the result to reference the call id.
    let messages = requests[1]["messages"].as_array().unwrap();
    let tool_msg = messages.iter().find(|m| m["role"] == "tool").unwrap();
    assert_eq!(tool_msg["tool_call_id"], "call_abc123");

    server.abort();
}

#[test]
fn conversation_round_trips_tool_calls() {
    let mut conversation = Conversation::new("test-model");
    conversation.add_message("user", "What is the weather in Paris?");
    conversation.messages.push(Message::assistant_tool_calls(
        "",
        vec![kowalski_core::conversation::ToolCall {
            id: "call_0".to_string(),
            function: kowalski_core::conversation::FunctionCall {
                name: "get_current_weather".to_string(),
                arguments: json!({"location": "Paris"}),
            },
        }],
    ));
    conversation
        .messages
        .push(Message::tool_result("call_0", "12 degrees and cloudy"));

    let exported = serde_json::to_string(&conversation).unwrap();
    let imported: Conversation = serde_json::from_str(&exported).unwrap();
    assert_eq!(imported.messages.len(), 3);
    let calls = imported.messages[1].tool_calls.as_ref().unwrap();
    assert_eq!(calls[0].function.arguments, json!({"location": "Paris"}));
    assert_eq!(imported.messages[2].tool_call_id.as_deref(), Some("call_0"));

    // Conversations persisted before native tool calling still load.
    let legacy = r#"{
        "id": "legacy",
        "model": "test-model",
        "messages": [
            { "role": "user", "content": "hi", "tool_calls": null },
            { "role": "assistant", "content": "hello" }
        ]
    }"#;
    let legacy: Conversation = serde_json::from_str(legacy).unwrap();
    assert_eq!(legacy.messages.len(), 2);
    assert!(legacy.messages[0].tool_calls.is_none());
    assert!(legacy.messages[1].tool_call_id.is_none());
}
