use log::{error, info, warn};
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
    pub search: SearchConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub api_key: Option<String>,
    pub provider: String,
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
            search: SearchConfig {
                api_key: None,
                provider: "duckduckgo".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();

        // 1. Try local config.toml first
        let local_config = std::path::Path::new("config.toml");
        if local_config.exists() {
            info!("Using local config.toml");
            builder = builder.add_source(config::File::from(local_config));
        } else {
            // 2. Try system config path
            let config_path = Self::get_config_path();
            if config_path.exists() {
                info!("Using system config at: {}", config_path.display());
                builder = builder.add_source(config::File::from(config_path));
            } else {
                warn!("No config file found, using defaults with environment overrides");
            }
        }

        // 3. Add environment variables (always checked, can override file settings)
        builder = builder.add_source(config::Environment::with_prefix("KOWALSKI"));

        // Build the config
        let settings = builder.build()?;

        // Try to deserialize the config, fall back to default if empty or invalid
        match settings.try_deserialize::<Config>() {
            Ok(config) => Ok(config),
            Err(_) => {
                let default_config = Config::default();
                if let Err(e) = default_config.save() {
                    error!("Warning: Could not save default config: {}", e);
                }
                Ok(default_config)
            }
        }
    }

    fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("kowalski");
        path.push("config.toml");
        println!("Config path: {}", &path.display());
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
