// Extracted from kowalski-federation/src/message.rs
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Types of messages that can be sent within the federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Register,
    TaskDelegation,
    TaskCompletion,
    Status,
    Error,
    Custom(String),
}

/// Message sent between federated agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationMessage {
    pub id: String,
    pub message_type: MessageType,
    pub sender: String,
    pub recipient: Option<String>,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: u64,
}

// Extracted from kowalski-federation/src/registry.rs

/// Registry for managing federated agents
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, FederatedAgentRef>>>,
}

impl AgentRegistry {
    pub async fn register_agent(&self, agent: FederatedAgentRef) -> Result<(), FederationError> { ... }
    pub async fn get_agent(&self, id: &str) -> Option<FederatedAgentRef> { ... }
    pub async fn list_agents(&self) -> Vec<(String, FederationRole)> { ... }
    pub async fn broadcast_message(&self, message: FederationMessage) -> Result<(), FederationError> { ... }
    pub async fn send_message(&self, recipient: &str, message: FederationMessage) -> Result<(), FederationError> { ... }
    pub async fn remove_agent(&self, id: &str) -> Result<(), FederationError> { ... }
}
