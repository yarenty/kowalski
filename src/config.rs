/// Config: The AI's settings, because apparently we need to customize everything.
/// "Configurations are like preferences - they're personal until they're wrong."
/// 
/// This module provides functionality for managing configuration settings.
/// Think of it as a settings menu for your AI, but without the annoying popups.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The main configuration struct that makes our AI feel special.
/// "Configs are like recipes - they work until you try to follow them."
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ollama: OllamaConfig,
    pub chat: ChatConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub default_model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatConfig {
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub stream: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama: OllamaConfig {
                base_url: "http://127.0.0.1:11434".to_string(),
                default_model: "mistral-small".to_string(),
            },
            chat: ChatConfig {
                temperature: Some(0.7),
                max_tokens: Some(512),
                stream: true,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config_path = Self::get_config_path();
        let mut settings = config::Config::builder()
            .add_source(config::File::from(config_path.clone()).required(false))
            .add_source(config::Environment::with_prefix("OLLAMA_AGENT"));

        // If no config file exists, use default config
        if !config_path.exists() {
            let default_config = Config::default();
            if let Err(e) = default_config.save() {
                eprintln!("Warning: Could not save default config: {}", e);
            }
            return Ok(default_config);
        }

        let settings = settings.build()?;
        let config: Config = settings.try_deserialize()?;
        Ok(config)
    }

    fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("ollama-agent");
        path.push("config.toml");
        path
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = Self::get_config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to serialize config: {}", e),
            )
        })?;

        std::fs::write(config_path, toml)
    }
} 