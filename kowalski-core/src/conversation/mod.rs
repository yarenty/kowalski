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
}

impl Conversation {
    pub fn new(model: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            model: model.to_string(),
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }

    pub fn get_messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
} 