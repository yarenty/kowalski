use super::ToolError;
use crate::tool::{Tool, ToolInput, ToolOutput, ToolParameter, ParameterType};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::json;
use std::sync::Arc;

pub struct WebScrapeTool {
    client: Arc<Client>,
}

impl WebScrapeTool {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Client::new()),
        }
    }
}

#[async_trait]
impl Tool for WebScrapeTool {
    fn name(&self) -> &str {
        "web_scrape"
    }

    fn description(&self) -> &str {
        "Scrapes content from web pages using CSS selectors"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "url".to_string(),
                description: "URL to scrape".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "selectors".to_string(),
                description: "CSS selectors to extract content from".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::Array,
            },
            ToolParameter {
                name: "follow_links".to_string(),
                description: "Whether to follow links to other pages".to_string(),
                required: false,
                default_value: Some("false".to_string()),
                parameter_type: ParameterType::Boolean,
            },
            ToolParameter {
                name: "max_depth".to_string(),
                description: "Maximum depth to follow links".to_string(),
                required: false,
                default_value: Some("2".to_string()),
                parameter_type: ParameterType::Number,
            },
        ]
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let params = input.parameters.as_object().ok_or_else(|| {
            ToolError::InvalidInput("Input parameters must be a JSON object".to_string())
        })?;

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing required parameter: url".to_string()))?
            .to_string();

        let selectors = params
            .get("selectors")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::InvalidInput("Missing required parameter: selectors".to_string()))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>();

        let follow_links = params
            .get("follow_links")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let max_depth = params
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        let result = self.scrape_page(&url, &selectors, follow_links, max_depth).await?;

        Ok(ToolOutput {
            result: json!(result),
            metadata: Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "url": url,
                "selectors": selectors,
                "follow_links": follow_links,
                "max_depth": max_depth,
            })),
        })
    }

    async fn scrape_page(
        &self,
        url: &str,
        selectors: &[String],
        follow_links: bool,
        max_depth: usize,
    ) -> Result<Vec<serde_json::Value>, ToolError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ToolError::Network(format!(
                "Failed to fetch URL {}: {}",
                url,
                response.status()
            )));
        }

        let body = response
            .text()
            .await
            .map_err(|e| ToolError::Network(e.to_string()))?;

        let document = Html::parse_document(&body);
        let mut results = Vec::new();

        for selector in selectors {
            let selector = Selector::parse(selector)
                .map_err(|e| ToolError::InvalidInput(format!("Invalid selector: {}", e)))?;

            let elements = document
                .select(&selector)
                .map(|element| {
                    let text = element
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ");
                    json!({
                        "selector": selector.to_string(),
                        "text": text.trim(),
                        "html": element.html(),
                    })
                })
                .collect::<Vec<_>>();

            results.extend(elements);
        }

        if follow_links && max_depth > 0 {
            let link_selector = Selector::parse("a[href]")
                .map_err(|e| ToolError::InvalidInput(format!("Failed to create link selector: {}", e)))?;

            let links = document
                .select(&link_selector)
                .filter_map(|element| {
                    element.value().attr("href").map(|href| {
                        url::Url::parse(&url)
                            .and_then(|base| base.join(href))
                            .map(|url| url.to_string())
                            .ok()
                    })
                })
                .collect::<Vec<_>>();

            for link in links {
                if let Ok(link_results) = self.scrape_page(&link, selectors, follow_links, max_depth - 1).await {
                    results.extend(link_results);
                }
            }
        }

        Ok(results)
    }
}
