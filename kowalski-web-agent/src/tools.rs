use async_trait::async_trait;
use kowalski_core::tools::{Tool, ToolOutput, TaskType, ToolInput};
use kowalski_core::config::Config;
use reqwest::Client;
use std::time::Duration;
use std::fmt;
use url::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Web-specific task types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebTaskType {
    /// Search the web
    Search,
    /// Browse a dynamic webpage
    BrowseDynamic,
    /// Scrape a static webpage
    ScrapeStatic,
}

impl fmt::Display for WebTaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TaskType for WebTaskType {
    fn name(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::BrowseDynamic => "browse_dynamic",
            Self::ScrapeStatic => "scrape_static",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Search => "Search the web using various search providers",
            Self::BrowseDynamic => "Browse and interact with dynamic web pages",
            Self::ScrapeStatic => "Scrape content from static web pages",
        }
    }
}

/// Represents different search providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchProvider {
    DuckDuckGo,
    Google,
    Bing,
}

impl fmt::Display for SearchProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            SearchProvider::DuckDuckGo => "duckduckgo",
            SearchProvider::Google => "google",
            SearchProvider::Bing => "bing",
        })
    }
}

/// A tool for searching the web
pub struct SearchTool {
    provider: SearchProvider,
    api_key: String,
}

impl SearchTool {
    pub fn new(provider: SearchProvider, api_key: String) -> Self {
        Self { provider, api_key }
    }
}

#[async_trait]
impl Tool for SearchTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        Ok(ToolOutput::new(
            json!({
                "results": format!("Search results for: {}", input.content),
                "provider": self.provider.to_string()
            }),
            Some(json!({
                "provider": self.provider.to_string(),
                "api_key_used": !self.api_key.is_empty(),
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        ))
    }
}

/// A tool for browsing dynamic web content
pub struct WebBrowser {
    client: Client,
    config: Config,
}

impl WebBrowser {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { client, config }
    }

    async fn fetch_dynamic_content(&self, url: &str) -> Result<String, String> {
        let response = self.client.get(url).send().await
            .map_err(|e| e.to_string())?;
        let text = response.text().await
            .map_err(|e| e.to_string())?;
        Ok(text)
    }
}

#[async_trait]
impl Tool for WebBrowser {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        // Validate URL
        Url::parse(&input.content).map_err(|e| e.to_string())?;
        
        // Fetch dynamic content
        let content = self.fetch_dynamic_content(&input.content).await?;
        
        Ok(ToolOutput::new(
            json!({
                "content": content,
                "url": input.content
            }),
            Some(json!({
                "content_type": "dynamic",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        ))
    }
}

/// A tool for scraping static web content
pub struct WebScraper {
    client: Client,
    config: Config,
}

impl WebScraper {
    pub fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { client, config }
    }

    async fn fetch_url(&self, url: &str) -> Result<String, String> {
        let response = self.client.get(url).send().await
            .map_err(|e| e.to_string())?;
        let text = response.text().await
            .map_err(|e| e.to_string())?;
        Ok(text)
    }
}

#[async_trait]
impl Tool for WebScraper {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        // Validate URL
        Url::parse(&input.content).map_err(|e| e.to_string())?;
        
        // Fetch content
        let content = self.fetch_url(&input.content).await?;
        
        Ok(ToolOutput::new(
            json!({
                "content": content,
                "url": input.content
            }),
            Some(json!({
                "content_type": "static",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_scraper() {
        let mut scraper = WebScraper::new(Config::default());
        let input = ToolInput::new(
            WebTaskType::ScrapeStatic.name().to_string(),
            "https://example.com".to_string(),
            json!({})
        );
        let result = scraper.execute(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_web_browser() {
        let mut browser = WebBrowser::new(Config::default());
        let input = ToolInput::new(
            WebTaskType::BrowseDynamic.name().to_string(),
            "https://example.com".to_string(),
            json!({})
        );
        let result = browser.execute(input).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_provider_display() {
        assert_eq!(SearchProvider::DuckDuckGo.to_string(), "duckduckgo");
        assert_eq!(SearchProvider::Google.to_string(), "google");
        assert_eq!(SearchProvider::Bing.to_string(), "bing");
    }
} 