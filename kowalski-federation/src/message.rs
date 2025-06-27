use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Types of messages that can be sent within the federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Registering with the federation
    Register,
    /// Task delegation
    TaskDelegation,
    /// Task completion
    TaskCompletion,
    /// Status update
    Status,
    /// Error report
    Error,
    /// Custom message type
    Custom(String),
}

/// Message sent between federated agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationMessage {
    /// Unique message ID
    pub id: String,
    /// Message type
    pub message_type: MessageType,
    /// Sender's ID
    pub sender: String,
    /// Optional recipient ID (None for broadcast)
    pub recipient: Option<String>,
    /// Content of the message
    pub content: String,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
    /// Timestamp
    pub timestamp: u64,
}

impl FederationMessage {
    /// Create a new federation message
    pub fn new(
        message_type: MessageType,
        sender: String,
        recipient: Option<String>,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type,
            sender,
            recipient,
            content,
            metadata,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
