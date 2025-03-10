/// Types module: Because we need more than just strings and numbers
/// "Type systems are like relationship counselors - they prevent a lot of mistakes before they happen." - A Type Theorist

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub temperature: f32,
    pub max_tokens: usize,
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamResponse {
    pub done: bool,
    pub message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl From<crate::conversation::Message> for Message {
    fn from(msg: crate::conversation::Message) -> Self {
        Self {
            role: msg.role,
            content: msg.content,
        }
    }
} 