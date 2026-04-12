use crate::error::KowalskiCliError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub agent_type: String, // web, academic, code, data
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub model: Option<String>,
    pub tools: Option<Vec<String>>,
    pub llm: Option<LLMConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl AgentConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, KowalskiCliError> {
        let content = fs::read_to_string(path)
            .map_err(|e| KowalskiCliError::Config(format!("Failed to read config file: {}", e)))?;

        let config: AgentConfig = toml::from_str(&content)
            .map_err(|e| KowalskiCliError::Config(format!("Failed to parse TOML config: {}", e)))?;

        Ok(config)
    }
}
