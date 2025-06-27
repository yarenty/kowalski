use kowalski_core::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAgentConfig {
    pub system_prompt: String,
}

impl Default for DataAgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are a data analysis assistant.".to_string(),
        }
    }
}

impl From<Config> for DataAgentConfig {
    fn from(_config: Config) -> Self {
        DataAgentConfig::default()
    }
}
