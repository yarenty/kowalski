/// Search module: Because Google is too mainstream.
/// "Search engines are like fortune tellers - they give you what you ask for, not what you want." - A Search Expert

/// Search Providers: Because one search engine is never enough
/// "In the grand scheme of things, all search engines are equally useless." - A Frustrated Developer

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::{Tool, ToolInput, ToolOutput, ToolError};

#[derive(Debug, Clone, PartialEq)]
pub enum SearchProvider {
    DuckDuckGo,
    Custom(String),
}

pub struct SearchTool {
    provider: SearchProvider,
    client: reqwest::Client,
}

impl SearchTool {
    pub fn new(provider: SearchProvider, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Kowalski/1.0")
            .build()
            .unwrap_or_default();

        Self { provider, client }
    }

    async fn search_duckduckgo(&self, query: &str) -> Result<Vec<SearchResult>, ToolError> {
        let url = format!("https://api.duckduckgo.com/?q={}&format=json", urlencoding::encode(query));
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(ToolError::RequestError)?;

        let data: DuckDuckGoResponse = response
            .json()
            .await
            .map_err(ToolError::JsonError)?;

        Ok(data.results.into_iter().map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.snippet,
        }).collect())
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search_tool"
    }

    fn description(&self) -> &str {
        "Searches the web using various providers, because one search engine is never enough"
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let query = input.query;
        
        let results = match self.provider {
            SearchProvider::DuckDuckGo => self.search_duckduckgo(&query).await?,
            SearchProvider::Custom(ref url) => {
                let response = self.client
                    .get(url)
                    .query(&[("q", &query)])
                    .send()
                    .await
                    .map_err(ToolError::RequestError)?;

                let data: Vec<SearchResult> = response
                    .json()
                    .await
                    .map_err(ToolError::JsonError)?;

                data
            }
        };

        Ok(ToolOutput {
            content: serde_json::to_string(&results).map_err(ToolError::JsonError)?,
            metadata: Default::default(),
            source: Some(format!("search:{:?}", self.provider)),
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