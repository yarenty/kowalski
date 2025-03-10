use chrono::{DateTime, Utc};
/// Conversation: The AI's memory, because apparently we need to remember things.
/// "Conversations are like diaries - they're personal but they're usually boring."
///
/// This module provides functionality for managing conversations with the AI.
/// Think of it as a diary for your AI, but without the teenage angst.
use serde::{Deserialize, Serialize};
use crate::agent::types::Message;


/// A conversation between the user and the AI, because apparently we need to be organized.
/// "Conversations are like relationships - they start simple but get complicated quickly."
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

impl Conversation {
    /// Creates a new conversation with the specified model.
    /// "Creating conversations is like starting a diary - it's exciting until you realize you have nothing to say."
    pub fn new(model: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            model: model.to_string(),
            created_at: Utc::now(),
        }
    }

    /// Adds a message to the conversation, because apparently we need to remember things.
    /// "Adding messages is like adding pages to a diary - it's therapeutic until someone reads it."
    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(Message {
            role: role.to_string(),
            content: content.to_string(),
            tool_calls: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the conversation creation
    /// "Testing conversations is like testing relationships - it's complicated and usually ends in tears."
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
