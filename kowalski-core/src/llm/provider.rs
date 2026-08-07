//! LLM abstraction used by [`crate::agent::BaseAgent`].
//!
//! ## Operator-facing errors (convention for implementors)
//!
//! Callers (including the HTTP `/api/chat` path) surface `KowalskiError` strings to operators and
//! CLI tools. When a request fails, prefer **actionable** messages over bare library errors:
//!
//! - **Include context**: which **API base / host**, **model id**, and **operation** (chat, embed,
//!   stream).
//! - **Add a short “What to check” list**: daemon or process up? `config.toml` `[llm]` keys
//!   (`provider`, `openai_api_base`, API key, Ollama host/port)? Network / VPN / TLS? Model
//!   spelling and `pull` / provider catalog?
//! - **Map HTTP / API bodies** when available (status + trimmed body), not only `Display` of the
//!   client error.
//!
//! Reference implementations: [`super::ollama::OllamaProvider`], [`super::openai::OpenAIProvider`].

use crate::conversation::{Message, ToolCall};
use crate::error::KowalskiError;
use crate::tools::{ParameterType, Tool};
use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::json;
use std::pin::Pin;

/// Incremental assistant text from [`LLMProvider::chat_stream`].
pub type TokenStream<'a> = Pin<Box<dyn Stream<Item = Result<String, KowalskiError>> + Send + 'a>>;

/// Wire-format tool declaration for native provider tool calling (OpenAI function format,
/// which Ollama shares): a name, a description, and a JSON Schema `parameters` object.
///
/// This is the **single owner** of the [`Tool`] metadata → wire format conversion
/// ([`ToolDefinition::from_tool`]); providers map it onto their client types.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema object describing the tool's arguments.
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// Build the wire declaration from a [`Tool`]'s metadata.
    pub fn from_tool(tool: &dyn Tool) -> Self {
        let mut properties = serde_json::Map::new();
        let mut required: Vec<String> = Vec::new();
        for param in tool.parameters() {
            let type_name = match param.parameter_type {
                ParameterType::String => "string",
                ParameterType::Number => "number",
                ParameterType::Boolean => "boolean",
                ParameterType::Array => "array",
                ParameterType::Object => "object",
            };
            properties.insert(
                param.name.clone(),
                json!({ "type": type_name, "description": param.description }),
            );
            if param.required {
                required.push(param.name);
            }
        }
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": properties,
                "required": required,
            }),
        }
    }

    /// The `{"type": "function", "function": {...}}` JSON both providers accept on the wire.
    pub fn wire_json(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// Result of a native tool-calling chat turn ([`LLMProvider::chat_with_tool_defs`]).
#[derive(Debug, Clone)]
pub enum ChatOutcome {
    /// Plain assistant text — no tool call requested.
    Text(String),
    /// The model requested one or more tool calls (possibly alongside assistant text).
    ToolCalls {
        content: Option<String>,
        calls: Vec<ToolCall>,
    },
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat request to the LLM.
    ///
    /// On failure, return [`KowalskiError::Server`] with an operator-oriented message (see module
    /// docs above).
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError>;

    /// Generate embeddings for the given text.
    ///
    /// On failure, return [`KowalskiError::Memory`] with an operator-oriented message.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError>;

    fn supports_streaming(&self) -> bool;

    /// Whether native (structured) tool calling may be used with the given model.
    ///
    /// Config-informed: implementations return the deployment's `[llm] native_tools`
    /// opt-in. Default `false` — callers must fall back to text-based tool prompting.
    fn supports_native_tools(&self, _model: &str) -> bool {
        false
    }

    /// Chat with native tool declarations. Tool-capable providers pass `tools` on the wire
    /// and surface structured [`ChatOutcome::ToolCalls`]; the caller executes the tools and
    /// sends results back as `role = "tool"` messages ([`Message::tool_result`]) in
    /// `messages` on the follow-up call.
    ///
    /// Default implementation ignores `tools` and behaves exactly like [`LLMProvider::chat`],
    /// so providers without tool support are unaffected.
    async fn chat_with_tool_defs(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<ChatOutcome, KowalskiError> {
        let _ = tools;
        self.chat(model, messages).await.map(ChatOutcome::Text)
    }

    /// Token deltas (concatenate for the full reply). Empty strings may be omitted by callers.
    ///
    /// Stream errors should follow the same clarity convention as [`LLMProvider::chat`].
    fn chat_stream(&self, model: &str, messages: Vec<Message>) -> TokenStream<'_>;
}

/// Single-chunk stream when a provider does not implement native token streaming.
pub fn chat_stream_single_chunk<'a>(
    llm: &'a (dyn LLMProvider + 'a),
    model: &'a str,
    messages: Vec<Message>,
) -> TokenStream<'a> {
    Box::pin(async_stream::stream! {
        match llm.chat(model, &messages).await {
            Ok(t) if !t.is_empty() => yield Ok(t),
            Ok(_) => {}
            Err(e) => yield Err(e),
        }
    })
}
