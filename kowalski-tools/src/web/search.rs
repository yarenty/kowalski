use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{ParameterType, ToolParameter};
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;

pub struct WebSearchTool {
    client: Arc<Client>,
    search_provider: Arc<String>,
}

impl WebSearchTool {
    pub fn new(search_provider: String) -> Self {
        Self {
            client: Arc::new(Client::new()),
            search_provider: Arc::new(search_provider),
        }
    }

    async fn duckduckgo_search(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<ToolOutput, String> {
        let url = format!("https://api.duckduckgo.com/?q={}&format=json", query);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let body = response.text().await.map_err(|e| e.to_string())?;

        let result = json!({
            "provider": "duckduckgo",
            "query": query,
            "results": body,
        });

        Ok(ToolOutput {
            result,
            metadata: Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "provider": "duckduckgo",
                "query": query,
                "num_results": num_results,
            })),
        })
    }

    async fn serper_search(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<ToolOutput, KowalskiError> {
        // Implementation for Serper API
        // Expects SERPER_API_KEY to be set in the environment
        let api_key = std::env::var("SERPER_API_KEY").map_err(|_| {
            KowalskiError::ToolConfig("SERPER_API_KEY environment variable not set".to_string())
        })?;
        let url = "https://google.serper.dev/search";
        let payload = json!({
            "q": query,
            "num": num_results
        });
        let response = self
            .client
            .post(url)
            .header("X-API-KEY", api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                KowalskiError::ToolExecution(format!("Serper API request failed: {}", e))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|e| {
            KowalskiError::ToolExecution(format!("Failed to read Serper API response: {}", e))
        })?;
        if !status.is_success() {
            return Err(KowalskiError::ToolExecution(format!(
                "Serper API error ({}): {}",
                status, body
            )));
        }
        let result = json!({
            "provider": "serper",
            "query": query,
            "results": body,
        });
        Ok(ToolOutput {
            result,
            metadata: Some(json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "provider": "serper",
                "query": query,
                "num_results": num_results,
            })),
        })
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        // Extract parameters
        let params = &input.parameters;
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| input.content.as_str());
        if query.is_empty() {
            return Err(KowalskiError::ToolExecution(
                "Missing 'query' parameter or content".to_string(),
            ));
        }
        let num_results = params
            .get("num_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(3);
        // Allow overriding provider via parameters, else use self.search_provider, else default to duckduckgo
        let provider = params
            .get("provider")
            .and_then(|v| v.as_str())
            .or_else(|| Some(self.search_provider.as_str()))
            .filter(|s| !s.is_empty())
            .unwrap_or("duckduckgo");
        match provider.to_lowercase().as_str() {
            "duckduckgo" => self
                .duckduckgo_search(query, num_results)
                .await
                .map_err(KowalskiError::ToolExecution),
            "serper" => self.serper_search(query, num_results).await,
            other => Err(KowalskiError::ToolConfig(format!(
                "Unknown search provider: {}",
                other
            ))),
        }
    }

    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Performs a web search using the configured provider."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "query".to_string(),
                description: "The search query string.".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "num_results".to_string(),
                description: "The number of search results to return (default: 3).".to_string(),
                required: false,
                default_value: Some("3".to_string()),
                parameter_type: ParameterType::Number,
            },
            ToolParameter {
                name: "provider".to_string(),
                description: "The search provider to use (e.g., 'duckduckgo', 'serper'). Default is 'duckduckgo'.".to_string(),
                required: false,
                default_value: Some("duckduckgo".to_string()),
                parameter_type: ParameterType::String,
            },
        ]
    }
}
