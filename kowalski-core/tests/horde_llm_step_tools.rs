//! Integration test: one horde LLM stage with `fs_tool` executed by `LlmStepHandler`
//! on both tool-loop paths — native structured tool calls and the ReAct text fallback —
//! against a mock Ollama server (real HTTP, real `TemplateAgent`, real `fs_tool`).

use axum::{Json, Router, extract::State, routing::post};
use kowalski_core::config::Config;
use kowalski_core::horde_step::{
    LlmStepHandler, NullEventSink, StepContext, StepHandler, StepOutcome, StepSpec,
};
use kowalski_core::template::agent::TemplateAgent;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

type CapturedRequests = Arc<Mutex<Vec<Value>>>;

fn messages_contain(body: &Value, predicate: impl Fn(&Value) -> bool) -> bool {
    body["messages"]
        .as_array()
        .is_some_and(|msgs| msgs.iter().any(predicate))
}

/// Mock Ollama: first turn asks for `fs_tool list_dir`, follow-up (tool result seen)
/// answers with the final stage text. Serves both wire shapes: structured `tool_calls`
/// when the request declares tools, JSON-in-text otherwise.
async fn mock_chat(
    State(captured): State<CapturedRequests>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let native = body["tools"].is_array();
    let follow_up = if native {
        messages_contain(&body, |m| m["role"] == "tool")
    } else {
        messages_contain(&body, |m| {
            m["content"]
                .as_str()
                .is_some_and(|c| c.contains("Based on the tool result"))
        })
    };
    captured.lock().unwrap().push(body);

    let message = if follow_up {
        json!({ "role": "assistant", "content": "# Stage report\n\nListing captured." })
    } else if native {
        json!({
            "role": "assistant",
            "content": "",
            "tool_calls": [
                { "function": { "name": "fs_tool", "arguments": { "task": "list_dir", "path": "." } } }
            ]
        })
    } else {
        json!({
            "role": "assistant",
            "content": "{\"name\": \"fs_tool\", \"parameters\": {\"task\": \"list_dir\", \"path\": \".\"}}"
        })
    };
    Json(json!({ "model": "test-model", "message": message, "done": true }))
}

/// Builds the horde fixture (agents/ + prompts/), the mock-backed `TemplateAgent`,
/// runs one `process` stage with `tool_ids = ["fs_tool"]`, and returns the captured
/// wire requests plus the artifact text.
async fn run_stage(native_tools: bool) -> (Vec<Value>, String) {
    let captured: CapturedRequests = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route("/api/chat", post(mock_chat))
        .with_state(captured.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let horde_root = tempfile::tempdir().unwrap();
    let workdir = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("marker-file.txt"), "present").unwrap();
    std::fs::create_dir_all(horde_root.path().join("agents")).unwrap();
    std::fs::create_dir_all(horde_root.path().join("prompts")).unwrap();
    std::fs::write(
        horde_root.path().join("agents").join("proc.md"),
        "---\nname = \"proc\"\nkind = \"process\"\nprompt_file = \"prompts/proc.md\"\noutput = \"debug/proc.md\"\ntool_ids = [\"fs_tool\"]\n---\n\n# Proc\n",
    )
    .unwrap();
    std::fs::write(
        horde_root.path().join("prompts").join("proc.md"),
        "List the project directory with fs_tool, then summarize.\n",
    )
    .unwrap();

    let memory_dir = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.ollama.host = addr.ip().to_string();
    config.ollama.port = addr.port();
    config.llm.native_tools = native_tools;
    config.memory.episodic_path = memory_dir.path().display().to_string();

    let agent = TemplateAgent::new(config).await.unwrap();
    let handler = LlmStepHandler::new(
        "process",
        Arc::new(tokio::sync::Mutex::new(agent)),
        "test-model",
    );

    let step = StepSpec {
        name: "proc".into(),
        kind: "process".into(),
        output: Some("debug/proc.md".into()),
        tool_ids: vec!["fs_tool".into()],
        ..Default::default()
    };
    let cancel = CancellationToken::new();
    let ctx = StepContext {
        run_id: "run-test",
        horde_id: "test-horde",
        step: &step,
        workdir: workdir.path(),
        horde_root: horde_root.path(),
        source: None,
        question: "",
        project_path: Some(project.path().to_path_buf()),
        previous_artifact: None,
        events: &NullEventSink,
        llm: None,
        tools: None,
        cancel: &cancel,
    };

    let StepOutcome::Completed { artifact, .. } = handler.execute(&ctx).await.unwrap();
    let artifact_text = std::fs::read_to_string(artifact.unwrap()).unwrap();

    server.abort();
    let requests = captured.lock().unwrap().clone();
    (requests, artifact_text)
}

fn tool_result_content(request: &Value, is_native: bool) -> String {
    let messages = request["messages"].as_array().unwrap();
    if is_native {
        messages
            .iter()
            .find(|m| m["role"] == "tool")
            .expect("tool-role message")["content"]
            .as_str()
            .unwrap()
            .to_string()
    } else {
        messages
            .iter()
            .rev()
            .find_map(|m| {
                m["content"]
                    .as_str()
                    .filter(|c| c.contains("Based on the tool result"))
            })
            .expect("spliced tool-result prompt")
            .to_string()
    }
}

#[tokio::test]
async fn horde_llm_stage_runs_fs_tool_on_native_path() {
    let (requests, artifact) = run_stage(true).await;

    assert_eq!(requests.len(), 2);
    assert!(
        requests[0]["tools"].is_array(),
        "native path declares tools on the wire"
    );
    assert!(
        requests[0]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["function"]["name"] == "fs_tool"),
        "stage allowlist exposes fs_tool"
    );
    // The real fs_tool ran against the sandboxed project dir.
    let result = tool_result_content(&requests[1], true);
    assert!(
        result.contains("marker-file.txt"),
        "fs_tool listed the project: {result}"
    );
    assert!(artifact.contains("Listing captured"));
}

#[tokio::test]
async fn horde_llm_stage_runs_fs_tool_on_react_path() {
    let (requests, artifact) = run_stage(false).await;

    assert_eq!(requests.len(), 2);
    assert!(
        requests.iter().all(|r| r["tools"].is_null()),
        "ReAct path never sends tools on the wire"
    );
    let result = tool_result_content(&requests[1], false);
    assert!(
        result.contains("marker-file.txt"),
        "fs_tool listed the project: {result}"
    );
    assert!(artifact.contains("Listing captured"));
}
