use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
pub enum FederationError {
    #[error("Agent {0} already exists in the federation")]
    DuplicateAgent(String),
    #[error("Agent {0} not found in the federation")]
    AgentNotFound(String),
    #[error("Failed to register with coordinator: {0}")]
    RegistrationFailed(String),
    #[error("Message delivery failed: {0}")]
    MessageDeliveryFailed(String),
    #[error("Invalid message type: {0}")]
    InvalidMessageType(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}
