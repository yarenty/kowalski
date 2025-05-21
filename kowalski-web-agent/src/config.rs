use crate::tools::SearchProvider;
use kowalski_core::config::{Config as CoreConfig, ConfigExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Web agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAgentConfig {
    /// Core configuration
    core: CoreConfig,
    /// Search configuration
    pub search: SearchConfig,
    /// Web scraping configuration
    pub scraping: ScrapingConfig,
}

impl Default for WebAgentConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            search: SearchConfig::default(),
            scraping: ScrapingConfig::default(),
        }
    }
}

impl ConfigExt for WebAgentConfig {
    fn core(&self) -> &CoreConfig {
        &self.core
    }

    fn core_mut(&mut self) -> &mut CoreConfig {
        &mut self.core
    }
}

/// Configuration for web search functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Default search provider
    pub default_provider: SearchProvider,
    /// API keys for different search providers
    pub api_keys: HashMap<SearchProvider, String>,
    /// Maximum number of search results
    pub max_results: usize,
    /// Additional search-specific settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_provider: SearchProvider::DuckDuckGo,
            api_keys: HashMap::new(),
            max_results: 10,
            additional: HashMap::new(),
        }
    }
}

/// Configuration for web scraping functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingConfig {
    /// User agent string
    pub user_agent: String,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    /// Additional scraping-specific settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            user_agent: "Kowalski/1.0".to_string(),
            request_timeout: 30,
            max_concurrent_requests: 5,
            additional: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_agent_config_default() {
        let config = WebAgentConfig::default();
        assert_eq!(config.search.default_provider, SearchProvider::DuckDuckGo);
        assert_eq!(config.scraping.request_timeout, 30);
    }

    #[test]
    fn test_config_ext() {
        let mut config = WebAgentConfig::default();
        config.set_additional("test_key", "test_value");
        let value: Option<String> = config.get_additional("test_key");
        assert_eq!(value, Some("test_value".to_string()));
    }
}
