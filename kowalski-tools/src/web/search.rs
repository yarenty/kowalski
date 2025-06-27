use super::ToolError;
use crate::tool::{Tool, ToolInput, ToolOutput, ToolParameter, ParameterType};
use async_trait::async_trait;
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
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Performs web searches using the specified search provider"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "query".to_string(),
                description: "Search query to perform".to_string(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "num_results".to_string(),
                description: "Number of results to return".to_string(),
                required: false,
                default_value: Some("5".to_string()),
                parameter_type: ParameterType::Number,
            },
        ]
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let params = input.parameters.as_object().ok_or_else(|| {
            ToolError::InvalidInput("Input parameters must be a JSON object".to_string())
        })?;

        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing required parameter: query".to_string()))?
            .to_string();

        let num_results = params
            .get("num_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        match self.search_provider.as_str() {
            "duckduckgo" => self.duckduckgo_search(&query, num_results).await,
            "serper" => self.serper_search(&query, num_results).await,
            _ => Err(ToolError::Config(format!(
                "Unsupported search provider: {}",
                self.search_provider
            ))),
        }
    }

    async fn duckduckgo_search(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<ToolOutput, ToolError> {
        let url = format!("https://api.duckduckgo.com/?q={}&format=json", query);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolError::Network(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| ToolError::Network(e.to_string()))?;

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
    ) -> Result<ToolOutput, ToolError> {
        // Implementation for Serper API would go here
        // This would require API key configuration
        Err(ToolError::Config("Serper API integration not implemented".to_string()))
    }
}
