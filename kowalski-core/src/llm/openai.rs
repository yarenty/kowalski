use super::provider::{ChatOutcome, LLMProvider, TokenStream, ToolDefinition};
use crate::conversation::{FunctionCall, Message, ToolCall};
use crate::error::KowalskiError;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        chat::{
            ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
            ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
            ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionTools,
            CreateChatCompletionRequestArgs, CreateChatCompletionResponse,
            FunctionCall as OpenAIFunctionCall, FunctionObject,
        },
        embeddings::CreateEmbeddingRequestArgs,
    },
};
use async_trait::async_trait;
use futures::StreamExt;

pub struct OpenAIProvider {
    client: Client<OpenAIConfig>,
    embedding_model: String,
    /// Effective HTTP API root (for operator-facing errors).
    api_base_display: String,
    native_tools: bool,
}

impl OpenAIProvider {
    /// `api_key` may be empty for some local OpenAI-compatible servers.
    /// `api_base` should be the full OpenAI API root (e.g. `https://api.openai.com/v1` or `http://localhost:1234/v1`).
    pub fn new(api_key: &str, api_base: Option<&str>) -> Self {
        let mut config = OpenAIConfig::new().with_api_key(api_key);
        let api_base_display = if let Some(base) = api_base {
            let trimmed = base.trim();
            if !trimmed.is_empty() {
                config = config.with_api_base(trimmed);
                trimmed.to_string()
            } else {
                "https://api.openai.com/v1".to_string()
            }
        } else {
            "https://api.openai.com/v1".to_string()
        };
        let client = Client::with_config(config);
        Self {
            client,
            embedding_model: "text-embedding-3-small".to_string(),
            api_base_display,
            native_tools: false,
        }
    }

    /// Opt in to native tool calling (`[llm] native_tools`); requires a tool-capable model.
    pub fn with_native_tools(mut self, enabled: bool) -> Self {
        self.native_tools = enabled;
        self
    }

    fn troubleshoot_chat(&self, model: &str, err: impl std::fmt::Display) -> String {
        format!(
            "OpenAI-compatible chat failed (model `{}`, API base `{}`): {}.\n\
             What to check:\n\
             - `config.toml` `[llm]` `provider = \"openai\"` and `openai_api_base` if you use a non-default host (must usually end with `/v1` for OpenAI-compatible HTTP APIs).\n\
             - **API key**: required for `api.openai.com`; many local servers accept an empty or placeholder key.\n\
             - **Model id**: must match the provider (e.g. `gpt-4o-mini`) or your local server’s model list.\n\
             - **Network**: VPN, firewall, corporate proxy, or TLS MITM breaking HTTPS.\n\
             - **Provider logs**: inspect the OpenAI-compatible server console for 4xx/5xx details.",
            model, self.api_base_display, err
        )
    }

    fn troubleshoot_embed(&self, err: impl std::fmt::Display) -> String {
        format!(
            "OpenAI-compatible embeddings failed (model `{}`, API base `{}`): {}.\n\
             What to check:\n\
             - Same connectivity and API key rules as chat.\n\
             - Embedding model id is valid for that provider (default here: `{}`).\n\
             - Local gateways: some require an explicit embeddings route or a different model name.",
            self.embedding_model, self.api_base_display, err, self.embedding_model
        )
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        let openai_messages = messages_to_openai(messages)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(openai_messages)
            .build()
            .map_err(|e| KowalskiError::Initialization(format!("OpenAI request error: {}", e)))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| KowalskiError::Server(self.troubleshoot_chat(model, &e)))?;

        let n_choices = response.choices.len();
        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| {
                let finish = response
                    .choices
                    .first()
                    .and_then(|c| c.finish_reason)
                    .map(|r| format!(" first_choice_finish_reason={:?}", r))
                    .unwrap_or_default();
                KowalskiError::Server(format!(
                    "No assistant text in OpenAI-compatible chat response (model `{}`, API base `{}`, {} choice(s){}).\n\
                     What to check: moderation or safety filters, `max_tokens` / empty completion, wrong model id, or a local server returning an unexpected schema.",
                    model, self.api_base_display, n_choices, finish
                ))
            })?;

        Ok(content)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(&self.embedding_model)
            .input(text)
            .build()
            .map_err(|e| KowalskiError::Initialization(format!("OpenAI embedding error: {}", e)))?;

        let response = self
            .client
            .embeddings()
            .create(request)
            .await
            .map_err(|e| KowalskiError::Memory(self.troubleshoot_embed(&e)))?;

        let n = response.data.len();
        let embedding = response
            .data
            .first()
            .map(|data| data.embedding.clone())
            .ok_or_else(|| {
                KowalskiError::Memory(format!(
                    "No embedding row in OpenAI-compatible response (embedding model `{}`, API base `{}`, {} row(s)).\n\
                     What to check: model supports embeddings on this provider, quota/rate limits, and response schema.",
                    self.embedding_model, self.api_base_display, n
                ))
            })?;

        Ok(embedding)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_native_tools(&self, _model: &str) -> bool {
        self.native_tools
    }

    async fn chat_with_tool_defs(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<ChatOutcome, KowalskiError> {
        let openai_messages = messages_to_openai(messages)?;

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(openai_messages)
            .tools(tool_defs_to_openai(tools))
            .build()
            .map_err(|e| KowalskiError::Initialization(format!("OpenAI request error: {}", e)))?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| KowalskiError::Server(self.troubleshoot_chat(model, &e)))?;

        outcome_from_response(&response).ok_or_else(|| {
            let finish = response
                .choices
                .first()
                .and_then(|c| c.finish_reason)
                .map(|r| format!(" first_choice_finish_reason={:?}", r))
                .unwrap_or_default();
            KowalskiError::Server(format!(
                "No assistant text or tool calls in OpenAI-compatible chat response (model `{}`, API base `{}`, {} choice(s){}).\n\
                 What to check: moderation or safety filters, `max_tokens` / empty completion, wrong model id, or a local server returning an unexpected schema.",
                model, self.api_base_display, response.choices.len(), finish
            ))
        })
    }

    fn chat_stream(&self, model: &str, messages: Vec<Message>) -> TokenStream<'_> {
        let openai_messages = match messages_to_openai(&messages) {
            Ok(m) => m,
            Err(e) => {
                return Box::pin(futures::stream::once(async move { Err(e) }));
            }
        };
        let request = match CreateChatCompletionRequestArgs::default()
            .model(model.to_string())
            .messages(openai_messages)
            .stream(true)
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                return Box::pin(futures::stream::once(async move {
                    Err(KowalskiError::Initialization(format!(
                        "OpenAI stream request: {e}"
                    )))
                }));
            }
        };
        let client = self.client.clone();
        let base = self.api_base_display.clone();
        let model_s = model.to_string();
        Box::pin(async_stream::stream! {
            let mut stream = match client.chat().create_stream(request).await {
                Ok(s) => s,
                Err(e) => {
                    yield Err(KowalskiError::Server(format!(
                        "OpenAI-compatible chat stream failed to start (model `{}`, API base `{}`): {}.\n\
                         What to check: same as non-stream chat — API base, key, model id, and that the server supports streaming for this model.",
                        model_s, base, e
                    )));
                    return;
                }
            };
            while let Some(item) = stream.next().await {
                match item {
                    Ok(resp) => {
                        for choice in resp.choices {
                            if let Some(ref c) = choice.delta.content
                                && !c.is_empty() {
                                    yield Ok(c.clone());
                                }
                        }
                    }
                    Err(e) => {
                        yield Err(KowalskiError::Server(format!(
                            "OpenAI-compatible chat stream chunk error (model `{}`, API base `{}`): {}.\n\
                             What to check: provider timeout, connection drop, or mid-stream API error; retry and inspect server logs.",
                            model_s, base, e
                        )));
                        return;
                    }
                }
            }
        })
    }
}

fn messages_to_openai(
    messages: &[Message],
) -> Result<Vec<ChatCompletionRequestMessage>, KowalskiError> {
    let mut openai_messages: Vec<ChatCompletionRequestMessage> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                openai_messages.push(ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
            "user" => {
                openai_messages.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(msg.content.clone())
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
            "assistant" => {
                let mut args = ChatCompletionRequestAssistantMessageArgs::default();
                if !msg.content.is_empty() {
                    args.content(msg.content.clone());
                }
                if let Some(ref calls) = msg.tool_calls {
                    args.tool_calls(tool_calls_to_openai(calls));
                }
                openai_messages.push(ChatCompletionRequestMessage::Assistant(
                    args.build().map_err(|e| {
                        KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                    })?,
                ));
            }
            "tool" => {
                let tool_call_id = msg.tool_call_id.clone().ok_or_else(|| {
                    KowalskiError::Initialization(
                        "Tool-role message has no tool_call_id (required by OpenAI-compatible APIs)"
                            .to_string(),
                    )
                })?;
                openai_messages.push(ChatCompletionRequestMessage::Tool(
                    ChatCompletionRequestToolMessageArgs::default()
                        .content(msg.content.clone())
                        .tool_call_id(tool_call_id)
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
            _ => {
                openai_messages.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(format!("[{}] {}", msg.role, msg.content))
                        .build()
                        .map_err(|e| {
                            KowalskiError::Initialization(format!("OpenAI message error: {}", e))
                        })?,
                ));
            }
        }
    }
    Ok(openai_messages)
}

fn tool_defs_to_openai(tools: &[ToolDefinition]) -> Vec<ChatCompletionTools> {
    tools
        .iter()
        .map(|def| {
            ChatCompletionTools::Function(ChatCompletionTool {
                function: FunctionObject {
                    name: def.name.clone(),
                    description: Some(def.description.clone()),
                    parameters: Some(def.parameters.clone()),
                    strict: None,
                },
            })
        })
        .collect()
}

/// OpenAI carries function arguments as a JSON **string**; our [`ToolCall`] carries a
/// [`serde_json::Value`] — these two convert between the representations.
fn tool_calls_to_openai(calls: &[ToolCall]) -> Vec<ChatCompletionMessageToolCalls> {
    calls
        .iter()
        .map(|call| {
            ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
                id: call.id.clone(),
                function: OpenAIFunctionCall {
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.to_string(),
                },
            })
        })
        .collect()
}

fn tool_calls_from_openai(calls: &[ChatCompletionMessageToolCalls]) -> Vec<ToolCall> {
    calls
        .iter()
        .filter_map(|call| match call {
            ChatCompletionMessageToolCalls::Function(f) => Some(ToolCall {
                id: f.id.clone(),
                function: FunctionCall {
                    name: f.function.name.clone(),
                    arguments: serde_json::from_str(&f.function.arguments).unwrap_or_else(|_| {
                        serde_json::Value::String(f.function.arguments.clone())
                    }),
                },
            }),
            ChatCompletionMessageToolCalls::Custom(_) => None,
        })
        .collect()
}

/// Map a Chat Completions response to a [`ChatOutcome`]; `None` when the first choice has
/// neither text nor tool calls.
fn outcome_from_response(response: &CreateChatCompletionResponse) -> Option<ChatOutcome> {
    let message = &response.choices.first()?.message;
    if let Some(ref raw_calls) = message.tool_calls {
        let calls = tool_calls_from_openai(raw_calls);
        if !calls.is_empty() {
            return Some(ChatOutcome::ToolCalls {
                content: message.content.clone().filter(|c| !c.is_empty()),
                calls,
            });
        }
    }
    message.content.clone().map(ChatOutcome::Text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn weather_tool() -> ToolDefinition {
        ToolDefinition {
            name: "get_current_weather".to_string(),
            description: "Get the current weather for a location".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string", "description": "The city name" }
                },
                "required": ["location"]
            }),
        }
    }

    #[test]
    fn request_with_tools_serializes_function_format() {
        let messages =
            messages_to_openai(&[Message::text("user", "What is the weather in Paris?")]).unwrap();
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(messages)
            .tools(tool_defs_to_openai(&[weather_tool()]))
            .build()
            .unwrap();
        let wire = serde_json::to_value(&request).unwrap();

        let tools = wire["tools"].as_array().expect("tools array on the wire");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "get_current_weather");
        assert_eq!(
            tools[0]["function"]["parameters"]["required"],
            json!(["location"])
        );
    }

    #[test]
    fn tool_role_and_assistant_tool_calls_wire_format() {
        let history = vec![
            Message::text("user", "What is the weather in Paris?"),
            Message::assistant_tool_calls(
                "",
                vec![ToolCall {
                    id: "call_abc123".to_string(),
                    function: FunctionCall {
                        name: "get_current_weather".to_string(),
                        arguments: json!({"location": "Paris"}),
                    },
                }],
            ),
            Message::tool_result("call_abc123", "12 degrees and cloudy"),
        ];
        let wire = serde_json::to_value(messages_to_openai(&history).unwrap()).unwrap();

        assert_eq!(wire[1]["role"], "assistant");
        assert_eq!(wire[1]["tool_calls"][0]["type"], "function");
        assert_eq!(wire[1]["tool_calls"][0]["id"], "call_abc123");
        assert_eq!(
            wire[1]["tool_calls"][0]["function"]["name"],
            "get_current_weather"
        );
        // Arguments cross the wire as a JSON string, per the Chat Completions contract.
        let args: serde_json::Value = serde_json::from_str(
            wire[1]["tool_calls"][0]["function"]["arguments"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(args, json!({"location": "Paris"}));

        assert_eq!(wire[2]["role"], "tool");
        assert_eq!(wire[2]["content"], "12 degrees and cloudy");
        assert_eq!(wire[2]["tool_call_id"], "call_abc123");
    }

    #[test]
    fn tool_role_message_without_call_id_is_an_error() {
        let mut bad = Message::tool_result("x", "result");
        bad.tool_call_id = None;
        assert!(messages_to_openai(&[bad]).is_err());
    }

    // Captured Chat Completions wire fixture: model answering with a tool call.
    #[test]
    fn response_with_tool_calls_parses_structured() {
        let fixture = r#"{
            "id": "chatcmpl-abc123",
            "object": "chat.completion",
            "created": 1770000000,
            "model": "gpt-4o-mini",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_current_weather",
                            "arguments": "{\"location\": \"Paris\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let response: CreateChatCompletionResponse = serde_json::from_str(fixture).unwrap();
        match outcome_from_response(&response).unwrap() {
            ChatOutcome::ToolCalls { content, calls } => {
                assert_eq!(content, None);
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].id, "call_abc123");
                assert_eq!(calls[0].function.name, "get_current_weather");
                assert_eq!(calls[0].function.arguments, json!({"location": "Paris"}));
            }
            other => panic!("expected ToolCalls, got {:?}", other),
        }
    }

    #[test]
    fn response_without_tool_calls_parses_text() {
        let fixture = r#"{
            "id": "chatcmpl-abc124",
            "object": "chat.completion",
            "created": 1770000000,
            "model": "gpt-4o-mini",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "It is sunny." },
                "finish_reason": "stop"
            }]
        }"#;
        let response: CreateChatCompletionResponse = serde_json::from_str(fixture).unwrap();
        match outcome_from_response(&response).unwrap() {
            ChatOutcome::Text(text) => assert_eq!(text, "It is sunny."),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn native_tools_flag_gates_support() {
        assert!(!OpenAIProvider::new("k", None).supports_native_tools("gpt-4o-mini"));
        assert!(
            OpenAIProvider::new("k", None)
                .with_native_tools(true)
                .supports_native_tools("gpt-4o-mini")
        );
    }
}
