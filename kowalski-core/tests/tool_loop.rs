//! Unit tests for the agent tool loop: native structured tool calls first, ReAct
//! JSON-in-text as fallback, driven by a mock provider that scripts both surfaces.

use async_trait::async_trait;
use kowalski_core::agent::{Agent, BaseAgent, MAX_TOOL_ITERATIONS};
use kowalski_core::config::{Config, ToolCallingMode};
use kowalski_core::conversation::{FunctionCall, Message, ToolCall};
use kowalski_core::error::KowalskiError;
use kowalski_core::llm::{ChatOutcome, LLMProvider, TokenStream, ToolDefinition};
use kowalski_core::memory::MemoryProvider;
use kowalski_core::memory::working::WorkingMemory;
use kowalski_core::tools::manager::ToolManager;
use kowalski_core::tools::{ParameterType, Tool, ToolInput, ToolOutput, ToolParameter};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Scripts `chat_with_tool_defs` outcomes and `chat` replies; records every request.
struct MockProvider {
    supports: bool,
    tool_outcomes: Mutex<VecDeque<ChatOutcome>>,
    chat_replies: Mutex<VecDeque<String>>,
    native_requests: Mutex<Vec<Vec<Message>>>,
    chat_requests: Mutex<Vec<Vec<Message>>>,
}

impl MockProvider {
    fn new(supports: bool) -> Arc<Self> {
        Arc::new(Self {
            supports,
            tool_outcomes: Mutex::new(VecDeque::new()),
            chat_replies: Mutex::new(VecDeque::new()),
            native_requests: Mutex::new(Vec::new()),
            chat_requests: Mutex::new(Vec::new()),
        })
    }

    fn script_outcomes(&self, outcomes: impl IntoIterator<Item = ChatOutcome>) {
        self.tool_outcomes.lock().unwrap().extend(outcomes);
    }

    fn script_replies(&self, replies: impl IntoIterator<Item = &'static str>) {
        self.chat_replies
            .lock()
            .unwrap()
            .extend(replies.into_iter().map(str::to_string));
    }

    fn native_requests(&self) -> Vec<Vec<Message>> {
        self.native_requests.lock().unwrap().clone()
    }

    fn chat_requests(&self) -> Vec<Vec<Message>> {
        self.chat_requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    async fn chat(&self, _model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        self.chat_requests.lock().unwrap().push(messages.to_vec());
        Ok(self
            .chat_replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| "script exhausted".to_string()))
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, KowalskiError> {
        Ok(vec![0.0])
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn chat_stream(&self, _model: &str, _messages: Vec<Message>) -> TokenStream<'_> {
        Box::pin(futures::stream::empty())
    }

    fn supports_native_tools(&self, _model: &str) -> bool {
        self.supports
    }

    async fn chat_with_tool_defs(
        &self,
        _model: &str,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<ChatOutcome, KowalskiError> {
        self.native_requests.lock().unwrap().push(messages.to_vec());
        Ok(self
            .tool_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| ChatOutcome::Text("script exhausted".to_string())))
    }
}

/// Records each execution's parameters; always answers `"echo-result"`.
#[derive(Clone)]
struct EchoTool {
    log: Arc<Mutex<Vec<Value>>>,
}

#[async_trait]
impl Tool for EchoTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        self.log.lock().unwrap().push(input.parameters);
        Ok(ToolOutput::new(json!("echo-result"), None))
    }

    fn name(&self) -> &str {
        "echo_tool"
    }

    fn description(&self) -> &str {
        "Echo test tool"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![ToolParameter {
            name: "x".to_string(),
            description: "value".to_string(),
            required: true,
            default_value: None,
            parameter_type: ParameterType::String,
        }]
    }
}

fn call(id: &str, x: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        function: FunctionCall {
            name: "echo_tool".to_string(),
            arguments: json!({ "x": x }),
        },
    }
}

fn tool_calls(calls: Vec<ToolCall>) -> ChatOutcome {
    ChatOutcome::ToolCalls {
        content: None,
        calls,
    }
}

fn memory() -> Arc<tokio::sync::Mutex<dyn MemoryProvider + Send + Sync>> {
    Arc::new(tokio::sync::Mutex::new(WorkingMemory::new(100)))
}

async fn agent_with(
    provider: Arc<MockProvider>,
    mode: ToolCallingMode,
    tool_log: Arc<Mutex<Vec<Value>>>,
) -> (BaseAgent, String) {
    let mut config = Config::default();
    config.llm.tool_calling = mode;
    let tool_manager = ToolManager::new();
    tool_manager.register(EchoTool { log: tool_log });
    let mut agent = BaseAgent::new(
        config,
        "test-agent",
        "tool loop test agent",
        provider,
        memory(),
        memory(),
        memory(),
        tool_manager,
    )
    .await
    .unwrap();
    let conv_id = agent.start_conversation("test-model");
    (agent, conv_id)
}

#[tokio::test]
async fn native_loop_tool_roundtrip() {
    let provider = MockProvider::new(true);
    provider.script_outcomes([
        tool_calls(vec![call("c1", "one")]),
        ChatOutcome::Text("done".to_string()),
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "run the tool", false, None)
        .await
        .unwrap();

    assert_eq!(result, "done");
    assert_eq!(log.lock().unwrap().len(), 1, "tool executed once");
    assert!(provider.chat_requests().is_empty(), "text path never used");

    let native = provider.native_requests();
    assert_eq!(native.len(), 2);
    // Follow-up request carries the assistant tool-call turn and the tool-role result.
    let follow_up = &native[1];
    let assistant = follow_up
        .iter()
        .find(|m| m.tool_calls.is_some())
        .expect("assistant tool-call turn in follow-up");
    assert_eq!(assistant.tool_calls.as_ref().unwrap()[0].id, "c1");
    let tool_msg = follow_up
        .iter()
        .find(|m| m.role == "tool")
        .expect("tool-role result in follow-up");
    assert_eq!(tool_msg.tool_call_id.as_deref(), Some("c1"));
    assert!(tool_msg.content.contains("echo-result"));

    // The structured turns are persisted in the conversation.
    let conversation = agent.get_conversation(&conv).unwrap();
    assert!(conversation.messages.iter().any(|m| m.role == "tool"));
    assert!(conversation.messages.iter().any(|m| m.tool_calls.is_some()));
    assert_eq!(conversation.messages.last().unwrap().content, "done");
}

#[tokio::test]
async fn native_loop_executes_multiple_calls_in_order() {
    let provider = MockProvider::new(true);
    provider.script_outcomes([
        tool_calls(vec![call("c1", "first"), call("c2", "second")]),
        ChatOutcome::Text("done".to_string()),
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "run both", false, None)
        .await
        .unwrap();

    assert_eq!(result, "done");
    let executed = log.lock().unwrap().clone();
    assert_eq!(executed.len(), 2);
    assert_eq!(executed[0]["x"], "first");
    assert_eq!(executed[1]["x"], "second");

    let follow_up = &provider.native_requests()[1];
    let tool_ids: Vec<_> = follow_up
        .iter()
        .filter(|m| m.role == "tool")
        .map(|m| m.tool_call_id.clone().unwrap())
        .collect();
    assert_eq!(tool_ids, vec!["c1", "c2"]);
}

#[tokio::test]
async fn native_loop_breaks_on_identical_consecutive_calls() {
    let provider = MockProvider::new(true);
    provider.script_outcomes([
        tool_calls(vec![call("c1", "same")]),
        tool_calls(vec![call("c2", "same")]),
        ChatOutcome::Text("never reached".to_string()),
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "loop forever", false, None)
        .await
        .unwrap();

    assert_eq!(result, "", "short-circuit returns without a final answer");
    assert_eq!(provider.native_requests().len(), 2);
    assert_eq!(
        log.lock().unwrap().len(),
        1,
        "second identical call set is not executed"
    );
}

#[tokio::test]
async fn native_loop_respects_iteration_cap() {
    let provider = MockProvider::new(true);
    provider.script_outcomes(
        (0..MAX_TOOL_ITERATIONS + 2).map(|i| tool_calls(vec![call("c", &format!("v{i}"))])),
    );
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "never stop", false, None)
        .await
        .unwrap();

    assert_eq!(result, "");
    assert_eq!(provider.native_requests().len(), MAX_TOOL_ITERATIONS);
    assert_eq!(log.lock().unwrap().len(), MAX_TOOL_ITERATIONS);
}

#[tokio::test]
async fn react_fallback_when_model_not_capable() {
    let provider = MockProvider::new(false);
    provider.script_replies([
        r#"{"name": "echo_tool", "parameters": {"x": "react"}}"#,
        "final answer",
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "use the tool", false, None)
        .await
        .unwrap();

    assert_eq!(result, "final answer");
    assert!(
        provider.native_requests().is_empty(),
        "native surface never used"
    );
    assert_eq!(provider.chat_requests().len(), 2);
    assert_eq!(log.lock().unwrap().len(), 1, "tool executed via ReAct");
}

#[tokio::test]
async fn react_breaker_on_repeated_call() {
    let provider = MockProvider::new(false);
    provider.script_replies([
        r#"{"name": "echo_tool", "parameters": {"x": "same"}}"#,
        r#"{"name": "echo_tool", "parameters": {"x": "same"}}"#,
        "never reached",
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "loop forever", false, None)
        .await
        .unwrap();

    assert_eq!(result, "", "ReAct breaker returns without a final answer");
    assert_eq!(provider.chat_requests().len(), 2);
    assert_eq!(log.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn react_loop_respects_iteration_cap() {
    let provider = MockProvider::new(false);
    // Distinct tool calls every turn: the breaker never fires, only the cap stops the loop.
    provider.chat_replies.lock().unwrap().extend(
        (0..MAX_TOOL_ITERATIONS + 2)
            .map(|i| format!(r#"{{"name": "echo_tool", "parameters": {{"x": "v{i}"}}}}"#)),
    );
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "never stop", false, None)
        .await
        .unwrap();

    assert_eq!(result, "");
    assert_eq!(provider.chat_requests().len(), MAX_TOOL_ITERATIONS);
    assert_eq!(log.lock().unwrap().len(), MAX_TOOL_ITERATIONS);
}

#[tokio::test]
async fn force_react_mode_overrides_capable_provider() {
    let provider = MockProvider::new(true);
    provider.script_replies(["plain text answer"]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::React, log).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "hello", false, None)
        .await
        .unwrap();

    assert_eq!(result, "plain text answer");
    assert!(provider.native_requests().is_empty());
    assert_eq!(provider.chat_requests().len(), 1);
}

#[tokio::test]
async fn force_native_mode_overrides_capability_flag() {
    let provider = MockProvider::new(false);
    provider.script_outcomes([ChatOutcome::Text("native anyway".to_string())]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Native, log).await;

    let result = agent
        .chat_with_tools_with_policy(&conv, "hello", false, None)
        .await
        .unwrap();

    assert_eq!(result, "native anyway");
    assert_eq!(provider.native_requests().len(), 1);
    assert!(provider.chat_requests().is_empty());
}

#[tokio::test]
async fn trait_chat_with_tools_uses_native_hook() {
    let provider = MockProvider::new(true);
    provider.script_outcomes([
        tool_calls(vec![call("c1", "via-trait")]),
        ChatOutcome::Text("hook works".to_string()),
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log.clone()).await;

    // The REPL entry point: the `Agent` trait method, not the policy variants.
    let result = Agent::chat_with_tools(&mut agent, &conv, "run the tool")
        .await
        .unwrap();

    assert_eq!(result, "hook works");
    assert_eq!(provider.native_requests().len(), 2);
    assert_eq!(log.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn stream_final_delivers_native_text_as_single_chunk() {
    let provider = MockProvider::new(true);
    provider.script_outcomes([
        tool_calls(vec![call("c1", "stream")]),
        ChatOutcome::Text("streamed final".to_string()),
    ]);
    let log = Arc::new(Mutex::new(Vec::new()));
    let (mut agent, conv) = agent_with(provider.clone(), ToolCallingMode::Auto, log).await;

    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    let result = agent
        .chat_with_tools_stream_final_with_options(&conv, "run the tool", &tx, false)
        .await
        .unwrap();
    drop(agent);

    assert_eq!(result, "streamed final");
    let mut chunks = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        chunks.push(chunk);
    }
    assert_eq!(chunks, vec!["streamed final"]);
}
