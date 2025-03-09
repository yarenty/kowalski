/// Search module: Because Google is too mainstream.
/// "Search engines are like fortune tellers - they give you what you ask for, not what you want." - A Search Expert

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use super::{Tool, ToolInput, ToolOutput, ToolError};
use log::debug;

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]  
#[allow(dead_code)]
pub enum SearchProvider {
    DuckDuckGo,
    Bing,
    Brave,
    Qwant,
    SearX,
    GoogleCustomSearch,
}

impl SearchProvider {
    pub fn get_base_url(&self) -> String {
        match self {
            SearchProvider::DuckDuckGo => "https://duckduckgo.com/html".to_string(),
            SearchProvider::Bing => "https://www.bing.com/search".to_string(),
            SearchProvider::Brave => "https://search.brave.com/search".to_string(),
            SearchProvider::Qwant => "https://www.qwant.com".to_string(),
            SearchProvider::SearX => "https://searx.be/search".to_string(),
            SearchProvider::GoogleCustomSearch => "https://www.googleapis.com/customsearch/v1".to_string(),
        }
    }

    pub fn get_query_param(&self) -> &str {
        match self {
            SearchProvider::DuckDuckGo => "q",
            SearchProvider::Bing => "q",
            SearchProvider::Brave => "q",
            SearchProvider::Qwant => "q",
            SearchProvider::SearX => "q",
            SearchProvider::GoogleCustomSearch => "q",
        }
    }

    pub fn requires_api_key(&self) -> bool {
        matches!(self, SearchProvider::GoogleCustomSearch)
    }
}

#[derive(Clone)]
pub struct SearchTool {
    provider: SearchProvider,
    api_key: String,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl SearchTool {
    pub fn new(provider: SearchProvider, api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .unwrap();
        
        Self { provider, api_key, client }
    }

    fn get_selectors(&self) -> (String, String, String) {
        match self.provider {
            SearchProvider::DuckDuckGo => (
                ".result__body,.nrn-react-div,.web-result".to_string(),
                ".result__title,.result__a".to_string(),
                ".result__snippet".to_string()
            ),
            SearchProvider::Bing => (
                ".b_algo".to_string(),
                "h2".to_string(),
                ".b_caption p".to_string()
            ),
            SearchProvider::Brave => (
                ".snippet".to_string(),
                ".title".to_string(),
                ".description".to_string()
            ),
            SearchProvider::Qwant => (
                ".web-result".to_string(),
                ".title".to_string(),
                ".desc".to_string()
            ),
            SearchProvider::SearX => (
                ".result".to_string(),
                ".result-title".to_string(),
                ".result-content".to_string()
            ),
            SearchProvider::GoogleCustomSearch => (
                ".g".to_string(),
                "h3".to_string(),
                ".snippet".to_string()
            ),
        }
    }

    async fn search(&self, query: &str) -> Result<String, Box<dyn std::error::Error>> {
        let base_url = self.provider.get_base_url();
        let query_param = self.provider.get_query_param();
        
        let mut params = vec![
            (query_param, query)
        ];

        if self.provider.requires_api_key() {
            params.push(("key", &self.api_key));
        }

        let response = self.client
            .get(&base_url)
            .query(&params)
            .send()
            .await?
            .text()
            .await?;

        Ok(response)
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
        let url = format!("{}/?{}={}",
            self.provider.get_base_url(),
            self.provider.get_query_param(),
            urlencoding::encode(&query));

        debug!("URL: {:?}", &url);
        let response = reqwest::get(&url)
            .await
            .map_err(ToolError::RequestError)?
            .text()
            .await
            .map_err(ToolError::RequestError)?;
        debug!("Response: {:?}", &response);

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