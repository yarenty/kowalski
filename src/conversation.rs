use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new(model: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            model: model.to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let model = "test-model";
        let conversation = Conversation::new(model);
        assert_eq!(conversation.model, model);
        assert!(conversation.messages.is_empty());
    }

    #[test]
    fn test_add_message() {
        let mut conversation = Conversation::new("test-model");
        conversation.add_message("user", "Hello");
        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].role, "user");
        assert_eq!(conversation.messages[0].content, "Hello");
    }
} 