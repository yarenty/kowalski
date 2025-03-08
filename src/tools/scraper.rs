/// Scraper module: Because websites are like puzzles - we need to take them apart.
/// "Web scraping is like cooking - you follow the recipe until you realize you're missing ingredients." - A Web Chef

use async_trait::async_trait;
use governor::{Quota, RateLimiter};
use scraper::{Html, Selector};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use super::{Tool, ToolInput, ToolOutput, ToolError};

pub struct WebScraper {
    client: reqwest::Client,
    rate_limiter: Arc<RateLimiter>,
    user_agent: String,
}

impl WebScraper {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Kowalski/1.0")
            .build()
            .unwrap_or_default();

        let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_second(
            NonZeroU32::new(1).unwrap(),
        )));

        Self {
            client,
            rate_limiter,
            user_agent: "Kowalski/1.0".to_string(),
        }
    }

    pub fn with_rate_limit(mut self, duration: Duration) -> Self {
        let requests_per_second = NonZeroU32::new(
            (1.0 / duration.as_secs_f32()).ceil() as u32
        ).unwrap_or(NonZeroU32::new(1).unwrap());

        self.rate_limiter = Arc::new(RateLimiter::direct(Quota::per_second(requests_per_second)));
        self
    }

    pub fn with_user_agent(mut self, user_agent: &str) -> Self {
        self.user_agent = user_agent.to_string();
        self.client = reqwest::Client::builder()
            .user_agent(&self.user_agent)
            .build()
            .unwrap_or_default();
        self
    }

    async fn scrape_url(&self, url: &str) -> Result<String, ToolError> {
        // Wait for rate limiter
        self.rate_limiter
            .until_ready()
            .await;

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(ToolError::RequestError)?;

        let html = response
            .text()
            .await
            .map_err(ToolError::RequestError)?;

        let document = Html::parse_document(&html);
        
        // Remove unwanted elements
        let unwanted_selectors = [
            "script",
            "style",
            "noscript",
            "iframe",
            "nav",
            "footer",
            "header",
            ".advertisement",
            "#cookie-notice",
        ];

        let mut content = html.clone();
        for selector in unwanted_selectors {
            if let Ok(sel) = Selector::parse(selector) {
                for element in document.select(&sel) {
                    content = content.replace(&element.html(), "");
                }
            }
        }

        Ok(content)
    }
}

#[async_trait]
impl Tool for WebScraper {
    fn name(&self) -> &str {
        "web_scraper"
    }

    fn description(&self) -> &str {
        "Scrapes web content with rate limiting and polite behavior, because manners matter"
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let url = input.query;
        let content = self.scrape_url(&url).await?;

        Ok(ToolOutput {
            content,
            metadata: Default::default(),
            source: Some(url),
        })
    }
} 