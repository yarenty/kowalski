use serde::{Deserialize, Serialize};
use kowalski_core::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateAgentConfig {
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,

    /// Timeout for requests in seconds
    pub request_timeout: u64,

    /// User agent string for requests
    pub user_agent: String,

    /// Whether to follow redirects
    pub follow_redirects: bool,

    /// Maximum number of redirects to follow
    pub max_redirects: usize,

    /// Whether to verify SSL certificates
    pub verify_ssl: bool,

    /// Proxy configuration (if any)
    pub proxy: Option<String>,

    /// System prompt for the agent
    pub system_prompt: String,

    /// Whether to enable debug logging
    pub debug_logging: bool,
}

impl Default for TemplateAgentConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10,
            request_timeout: 30,
            user_agent: "Kowalski Agent/1.0".to_string(),
            follow_redirects: true,
            max_redirects: 5,
            verify_ssl: true,
            proxy: None,
            system_prompt: "You are a helpful assistant.".to_string(),
            debug_logging: false,
        }
    }
}

impl From<Config> for TemplateAgentConfig {
    fn from(_config: Config) -> Self {
        // Use only defaults for now, as config.agent.* does not exist
        TemplateAgentConfig::default()
    }
} 