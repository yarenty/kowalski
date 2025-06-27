use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::json;
use std::sync::Arc;

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
                if let Ok(link_results) =
                    Box::pin(self.scrape_page(&link, selectors, follow_links, max_depth - 1)).await
                {
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
                        let text = element.text().collect::<Vec<_>>().join(" ");
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
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // Parse parameters from input
        let url = input
            .parameters
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| input.content.as_str());
        if url.is_empty() {
            return Err(KowalskiError::ToolExecution(
                "Missing 'url' parameter or content".to_string(),
            ));
        }
        let selectors = input
            .parameters
            .get("selectors")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                KowalskiError::ToolExecution(
                    "Missing or invalid 'selectors' parameter (must be array of strings)"
                        .to_string(),
                )
            })?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        if selectors.is_empty() {
            return Err(KowalskiError::ToolExecution(
                "'selectors' parameter must contain at least one selector".to_string(),
            ));
        }
        let follow_links = input
            .parameters
            .get("follow_links")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let max_depth = input
            .parameters
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

        let results = self
            .scrape_page(url, &selectors, follow_links, max_depth)
            .await
            .map_err(KowalskiError::ToolExecution)?;

        let metadata = serde_json::json!({
            "url": url,
            "selectors": selectors,
            "follow_links": follow_links,
            "max_depth": max_depth,
        });

        Ok(ToolOutput {
            result: serde_json::Value::Array(results),
            metadata: Some(metadata),
        })
    }

    fn name(&self) -> &str {
        "web_scrape"
    }

    fn description(&self) -> &str {
        "Scrapes web pages for content using CSS selectors."
    }

    fn parameters(&self) -> Vec<kowalski_core::tools::ToolParameter> {
        use kowalski_core::tools::{ParameterType, ToolParameter};
        vec![
            ToolParameter {
                name: "url".to_string(),
                description: "The URL of the web page to scrape. If not provided, the 'content' field will be used as the URL.".to_string(),
                required: false,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "selectors".to_string(),
                description: "A list of CSS selectors to extract content from the page. Each selector should be a string.".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::Array,
            },
            ToolParameter {
                name: "follow_links".to_string(),
                description: "Whether to follow links found on the page and scrape them recursively. Default is false.".to_string(),
                required: false,
                default_value: Some("false".to_string()),
                parameter_type: ParameterType::Boolean,
            },
            ToolParameter {
                name: "max_depth".to_string(),
                description: "The maximum recursion depth for following links. Default is 1 (only the initial page).".to_string(),
                required: false,
                default_value: Some("1".to_string()),
                parameter_type: ParameterType::Number,
            },
        ]
    }
}
