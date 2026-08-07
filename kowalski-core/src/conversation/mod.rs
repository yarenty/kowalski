use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Conversation: The AI's memory of what it's been talking about.
/// "Conversations are like dreams - they make sense at the time but are hard to explain later."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub model: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    /// Structured calls emitted by an assistant turn (native provider tool calling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// For `role = "tool"` messages: the id of the [`ToolCall`] this result answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn text(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Assistant turn that requested tool calls (content may be empty).
    pub fn assistant_tool_calls(content: &str, calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_calls: Some(calls),
            tool_call_id: None,
        }
    }

    /// Tool result answering the call with the given id, sent back as `role = "tool"`.
    pub fn tool_result(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

impl Conversation {
    pub fn new(model: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            model: model.to_string(),
            messages: Vec::new(),
        }
    }

    pub fn with_id(model: &str, id: &str) -> Self {
        Self {
            id: id.to_string(),
            model: model.to_string(),
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message::text(role, content));
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
