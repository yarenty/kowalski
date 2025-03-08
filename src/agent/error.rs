/// Error types for agents, because what's life without a little failure?
/// "Errors are like spices - they make the success taste better." - A Philosophical Developer

use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum AgentError {
    RequestError(reqwest::Error),
    JsonError(serde_json::Error),
    ServerError(String),
    ConfigError(config::ConfigError),
    IoError(std::io::Error),
    ToolError(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::RequestError(e) => write!(f, "Request error: {}", e),
            AgentError::JsonError(e) => write!(f, "JSON error: {}", e),
            AgentError::ServerError(e) => write!(f, "Server error: {}", e),
            AgentError::ConfigError(e) => write!(f, "Config error: {}", e),
            AgentError::IoError(e) => write!(f, "IO error: {}", e),
            AgentError::ToolError(e) => write!(f, "Tool error: {}", e),
        }
    }
}

impl Error for AgentError {} 