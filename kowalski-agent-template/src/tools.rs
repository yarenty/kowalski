use crate::config::TemplateAgentConfig;
use kowalski_core::error::KowalskiError;
// use kowalski_tools::{Tool, ToolInput, ToolOutput}; // Remove or update this line
use reqwest::Client;
use std::time::Duration;

/// A base HTTP client tool that can be extended by specialized agents
pub struct HttpClientTool {
    client: Client,
    config: TemplateAgentConfig,
}

impl HttpClientTool {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            config: TemplateAgentConfig::default(),
        }
    }

    pub fn with_config(config: TemplateAgentConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout))
            .user_agent(&config.user_agent)
            .pool_max_idle_per_host(config.max_concurrent_requests)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub async fn get(&self, url: &str) -> Result<String, KowalskiError> {
        let response = self.client.get(url).send().await?;
        let text = response.text().await?;
        Ok(text)
    }

    pub async fn post(&self, url: &str, body: &str) -> Result<String, KowalskiError> {
        let response = self.client.post(url).body(body.to_string()).send().await?;
        let text = response.text().await?;
        Ok(text)
    }
}

/// A base content processing tool that can be extended by specialized agents
pub struct ContentProcessor {
    config: TemplateAgentConfig,
}

impl ContentProcessor {
    pub fn new() -> Self {
        Self {
            config: TemplateAgentConfig::default(),
        }
    }

    pub fn with_config(config: TemplateAgentConfig) -> Self {
        Self { config }
    }

    pub fn process_content(&self, content: &str) -> Result<String, KowalskiError> {
        // Base content processing logic that can be extended
        Ok(content.to_string())
    }
}

/// A base search tool that can be extended by specialized agents
pub struct SearchTool {
    config: TemplateAgentConfig,
}

impl SearchTool {
    pub fn new() -> Self {
        Self {
            config: TemplateAgentConfig::default(),
        }
    }

    pub fn with_config(config: TemplateAgentConfig) -> Self {
        Self { config }
    }

    pub async fn search(&self, query: &str) -> Result<String, KowalskiError> {
        // Base search logic that can be extended
        Ok(format!("Search results for: {}", query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client() {
        let client = HttpClientTool::new();
        let result = client.get("https://example.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_content_processor() {
        let processor = ContentProcessor::new();
        let result = processor.process_content("test content");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_tool() {
        let search = SearchTool::new();
        let result = search.search("test query").await;
        assert!(result.is_ok());
    }
} 