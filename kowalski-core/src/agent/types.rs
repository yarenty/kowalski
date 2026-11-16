use crate::conversation::Message;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    /// Ollama reads generation settings only from here; top-level ones are ignored.
    #[serde(default, skip_serializing_if = "ChatOptions::is_empty")]
    pub options: ChatOptions,
    pub tools: Option<serde_json::Value>,
    /// Ollama constrained decoding: a JSON Schema the response must conform to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<serde_json::Value>,
}

/// The `options` object of an Ollama `/api/chat` request. An unset field is left to Ollama.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChatOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Context window in tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,
}

impl ChatOptions {
    pub fn is_empty(&self) -> bool {
        self.temperature.is_none() && self.num_ctx.is_none()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamResponse {
    pub done: bool,
    pub message: Message,
}
