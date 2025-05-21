use kowalski_core::config::{Config as CoreConfig, ConfigExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Academic agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcademicAgentConfig {
    /// Core configuration
    core: CoreConfig,
    /// Academic search configuration
    pub search: AcademicSearchConfig,
    /// Paper parsing configuration
    pub parsing: PaperParsingConfig,
}

impl Default for AcademicAgentConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            search: AcademicSearchConfig::default(),
            parsing: PaperParsingConfig::default(),
        }
    }
}

impl From<CoreConfig> for AcademicAgentConfig {
    fn from(config: CoreConfig) -> Self {
        let mut academic_config = Self::default();
        academic_config.core = config;
        academic_config
    }
}

impl ConfigExt for AcademicAgentConfig {
    fn core(&self) -> &CoreConfig {
        &self.core
    }

    fn core_mut(&mut self) -> &mut CoreConfig {
        &mut self.core
    }
}

/// Configuration for academic search functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcademicSearchConfig {
    /// Default academic search provider
    pub default_provider: String,
    /// API keys for different academic search providers
    pub api_keys: HashMap<String, String>,
    /// Maximum number of search results
    pub max_results: usize,
    /// Whether to include full text in search results
    pub include_full_text: bool,
    /// Additional search-specific settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for AcademicSearchConfig {
    fn default() -> Self {
        Self {
            default_provider: "semantic_scholar".to_string(),
            api_keys: HashMap::new(),
            max_results: 20,
            include_full_text: false,
            additional: HashMap::new(),
        }
    }
}

/// Configuration for paper parsing functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperParsingConfig {
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Supported file formats
    pub supported_formats: Vec<String>,
    /// Whether to extract figures
    pub extract_figures: bool,
    /// Whether to extract tables
    pub extract_tables: bool,
    /// Additional parsing-specific settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl Default for PaperParsingConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            supported_formats: vec!["pdf".to_string(), "txt".to_string()],
            extract_figures: true,
            extract_tables: true,
            additional: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_academic_agent_config_default() {
        let config = AcademicAgentConfig::default();
        assert_eq!(config.search.default_provider, "semantic_scholar");
        assert_eq!(config.parsing.max_file_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_config_ext() {
        let mut config = AcademicAgentConfig::default();
        config.set_additional("test_key", "test_value");
        let value: Option<String> = config.get_additional("test_key");
        assert_eq!(value, Some("test_value".to_string()));
    }
}
