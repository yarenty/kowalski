use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::json;
use std::sync::Arc;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_core::error::KowalskiError;

pub struct WebScrapeTool {
    client: Arc<Client>,
}

impl Default for WebScrapeTool {
       fn default() -> Self {
           Self::new()
       }
 }

impl WebScrapeTool {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Client::new()),
        }
    }

    async fn scrape_page(
        &self,
        url: &str,
        selectors: &[String],
        follow_links: bool,
        max_depth: usize,
    ) -> Result<Vec<serde_json::Value>, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL {}: {}", url, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch URL {}: {}",
                url,
                response.status()
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        let mut results = Self::extract_from_html(&body, selectors);

        if follow_links && max_depth > 0 {
            let links = Self::extract_links(&body, url)?;
            for link in links {
                if let Ok(link_results) = Box::pin(self.scrape_page(&link, selectors, follow_links, max_depth - 1)).await {
                    results.extend(link_results);
                }
            }
        }

        Ok(results)
    }

    fn extract_from_html(body: &str, selectors: &[String]) -> Vec<serde_json::Value> {
        let document = Html::parse_document(body);
        let mut results = Vec::new();
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                let elements = document
                    .select(&selector)
                    .map(|element| {
                        let text = element
                            .text()
                            .collect::<Vec<_>>()
                            .join(" ");
                        json!({
                            "selector": selector_str,
                            "text": text.trim(),
                            "html": element.html(),
                        })
                    })
                    .collect::<Vec<_>>();
                results.extend(elements);
            }
        }
        results
    }

    fn extract_links(body: &str, base_url: &str) -> Result<Vec<String>, String> {
        let document = Html::parse_document(body);
        let link_selector = Selector::parse("a[href]")
            .map_err(|e| format!("Failed to create link selector: {}", e))?;
        let links = document
            .select(&link_selector)
            .filter_map(|element| {
                element.value().attr("href").and_then(|href| {
                    url::Url::parse(base_url)
                        .and_then(|base| base.join(href))
                        .map(|url| url.to_string())
                        .ok()
                })
            })
            .collect::<Vec<_>>();
        Ok(links)
    }
}

#[async_trait]
impl Tool for WebScrapeTool {
    async fn execute(&mut self, _input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // ... implement or stub ...
        Err(KowalskiError::ToolExecution("Not implemented".to_string()))
    }
    fn name(&self) -> &str {
        "web_scrape"
    }
    fn description(&self) -> &str {
        "Scrapes web pages for content using CSS selectors."
    }
    fn parameters(&self) -> Vec<kowalski_core::tools::ToolParameter> {
        vec![]
    }
}
