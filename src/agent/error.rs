/// Error types for agents, because what's life without a little failure?
/// "Errors are like spices - they make the success taste better." - A Philosophical Developer

use std::fmt;
use std::error::Error;
use crate::utils::{PdfReaderError, PaperCleanerError};
use reqwest;
use config;

#[derive(Debug)]
pub enum AgentError {
    Request(reqwest::Error),
    Json(serde_json::Error),
    Server(String),
    Config(String),
    Io(std::io::Error),
    Tool(String),
    Serialization(String),
    ConversationNotFound(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::Request(e) => write!(f, "Request error: {}", e),
            AgentError::Json(e) => write!(f, "JSON error: {}", e),
            AgentError::Server(e) => write!(f, "Server error: {}", e),
            AgentError::Config(e) => write!(f, "Config error: {}", e),
            AgentError::Io(e) => write!(f, "IO error: {}", e),
            AgentError::Tool(e) => write!(f, "Tool error: {}", e),
            AgentError::Serialization(e) => write!(f, "Serialization error: {}", e),
            AgentError::ConversationNotFound(e) => write!(f, "Conversation not found: {}", e),
        }
    }
}

impl Error for AgentError {}

impl From<PdfReaderError> for AgentError {
    fn from(err: PdfReaderError) -> Self {
        AgentError::Tool(err.to_string())
    }
}

impl From<PaperCleanerError> for AgentError {
    fn from(err: PaperCleanerError) -> Self {
        AgentError::Tool(err.to_string())
    }
}

impl From<reqwest::Error> for AgentError {
    fn from(err: reqwest::Error) -> Self {
        AgentError::Request(err)
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::Json(err)
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::Io(err)
    }
}

impl From<config::ConfigError> for AgentError {
    fn from(err: config::ConfigError) -> Self {
        AgentError::Config(err.to_string())
    }
} 