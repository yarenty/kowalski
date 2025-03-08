/// Browser module: Because sometimes we need to pretend we're human.
/// "Web browsers are like cats - they do what they want and ignore your preferences." - A Web Developer

use async_trait::async_trait;
use fantoccini::{Client, ClientBuilder};
use scraper::{Html, Selector};
use super::{Tool, ToolInput, ToolOutput, ToolError};
use std::time::Duration;

pub struct WebBrowser {
    client: reqwest::Client,
    headless: Option<Client>,
    user_agent: String,
}

impl WebBrowser {
    pub fn new(config: crate::config::Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Kowalski/1.0")
            .build()
            .unwrap_or_default();

        Self {
            client,
            headless: None,
            user_agent: "Kowalski/1.0".to_string(),
        }
    }

    pub async fn init_headless(&mut self) -> Result<(), ToolError> {
        let caps = serde_json::json!({
            "browserName": "firefox",
            "moz:firefoxOptions": {
                "args": ["--headless"]
            }
        });

        self.headless = Some(
            ClientBuilder::native()
                .capabilities(caps)
                .connect("http://localhost:4444")
                .await
                .map_err(|e| ToolError::BrowserError(e.to_string()))?,
        );

        Ok(())
    }

    async fn extract_content(&self, html: &str) -> Result<String, ToolError> {
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
        let body_selector = Selector::parse("body").map_err(|e| ToolError::ScrapingError(e.to_string()))?;
        if let Some(body) = document.select(&body_selector).next() {
            Ok(body.text().collect::<Vec<_>>().join(" "))
        } else {
            Err(ToolError::ScrapingError("No content found".to_string()))
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

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let url = input.query;
        
        // Try simple GET request first
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(ToolError::RequestError)?;

        let html = response
            .text()
            .await
            .map_err(ToolError::RequestError)?;

        let content = self.extract_content(&html).await?;

        Ok(ToolOutput {
            content,
            metadata: Default::default(),
            source: Some(url),
        })
    }
}