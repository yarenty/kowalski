/// Search module: Because Google is too mainstream.
/// "Search engines are like fortune tellers - they give you what you ask for, not what you want." - A Search Expert

/// Search Providers: Because one search engine is never enough
/// "In the grand scheme of things, all search engines are equally useless." - A Frustrated Developer

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::{Tool, ToolInput, ToolOutput, ToolError};

#[derive(Debug, Clone)]
pub enum SearchProvider {
    DuckDuckGo,
    Custom(String),
}

pub struct SearchTool {
    provider: SearchProvider,
    api_key: String,
}

impl SearchTool {
    pub fn new(provider: SearchProvider, api_key: String) -> Self {
        Self {
            provider,
            api_key,
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Searches the web using various search providers"
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let query = input.query;
        let url = format!("https://api.duckduckgo.com/?q={}&format=json", urlencoding::encode(&query));

        let response = reqwest::get(&url)
            .await
            .map_err(ToolError::RequestError)?
            .text()
            .await
            .map_err(ToolError::RequestError)?;

        Ok(ToolOutput {
            content: response,
            metadata: Default::default(),
            source: Some(url),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    results: Vec<DuckDuckGoResult>,
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoResult {
    title: String,
    url: String,
    snippet: String,
}