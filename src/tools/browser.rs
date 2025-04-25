/// Browser module: Because sometimes we need to pretend we're human.
/// "Web browsers are like cats - they do what they want and ignore your preferences." - A Web Developer
use async_trait::async_trait;
// use fantoccini::{Client, ClientBuilder};
use super::{Tool, ToolInput, ToolOutput};
use scraper::{Html, Selector};
use std::time::Duration;
// use serde_json::{json, Value};
use crate::utils::KowalskiError;
use log::{debug, info};
/// WebBrowser: Because sometimes we need to pretend we're human.
pub struct WebBrowser {
    client: reqwest::Client,
    user_agent: String,
    config: crate::config::Config,
}

impl Clone for WebBrowser {
    fn clone(&self) -> Self {
        Self {
            client: reqwest::Client::new(),
            user_agent: self.user_agent.clone(),
            config: self.config.clone(),
        }
    }
}

impl WebBrowser {
    pub fn new(_config: crate::config::Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Kowalski/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        info!("Client: {:?}", &client);

        Self {
            client,
            user_agent: "Kowalski/1.0".to_string(),
            config: _config,
        }
    }

    async fn extract_content(&self, html: &str) -> Result<String, KowalskiError> {
        let document = Html::parse_document(html);

        // Try to find main content
        let selectors = [
            "article",
            "main",
            ".content",
            "#content",
            ".post-content",
            ".article-content",
        ];

        for selector in selectors {
            if let Ok(sel) = Selector::parse(selector) {
                if let Some(element) = document.select(&sel).next() {
                    return Ok(element.text().collect::<Vec<_>>().join(" "));
                }
            }
        }

        // Fallback to body
        let body_selector =
            Selector::parse("body").map_err(|e| KowalskiError::Scraping(e.to_string()))?;
        if let Some(body) = document.select(&body_selector).next() {
            Ok(body.text().collect::<Vec<_>>().join(" "))
        } else {
            Err(KowalskiError::Scraping("No content found".to_string()))
        }
    }
}

#[async_trait]
impl Tool for WebBrowser {
    fn name(&self) -> &str {
        "web_browser"
    }

    fn description(&self) -> &str {
        "Fetches and processes web pages, because copy-pasting is too mainstream"
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let url = input.query;

        debug!("Executing web browser tool with URL: {}", url);
        // Try simple GET request first
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(KowalskiError::Request)?;
        debug!("Response: {:?}", &response);

        let html = response.text().await.map_err(KowalskiError::Request)?;
        debug!("HTML: {:?}", &html);

        let content = self.extract_content(&html).await?;
        debug!("Content: {:?}", &content);

        Ok(ToolOutput {
            content,
            metadata: Default::default(),
            source: Some(url),
        })
    }
}
