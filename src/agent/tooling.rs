/// Tooling Agent: Because browsing the web manually is so last century.
/// "Web scraping is like archaeology - you dig through layers of HTML hoping to find treasure." - A Digital Archaeologist

use async_trait::async_trait;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::config::Config;
use crate::conversation::{Conversation, Message};
use crate::role::Role;
use super::{Agent, AgentError, BaseAgent};
use super::types::{ChatRequest, StreamResponse};
use crate::tools::{Tool, ToolChain, WebBrowser, SearchTool, WebScraper, ToolCache, Storage, ToolInput, ToolOutput};
use crate::tools::search::SearchProvider;


/// ToolingAgent: Your personal web crawler with a sense of humor.
pub struct ToolingAgent {
    base: BaseAgent,
    chain: ToolChain,
    cache: ToolCache,
}

#[async_trait]
impl Agent for ToolingAgent {
    fn new(config: Config) -> Result<Self, AgentError> {
        let base = BaseAgent::new(
            config.clone(),
            "Tooling Agent",
            "A versatile agent that uses various tools to process information",
        )?;

        let mut chain = ToolChain::new();
        chain.add_tool(Box::new(WebBrowser::new(config.clone())));
        chain.add_tool(Box::new(SearchTool::new(
            SearchProvider::DuckDuckGo,
            config.search.api_key.clone().unwrap_or_default(),
        )));
        chain.add_tool(Box::new(WebScraper::new()));

        let cache = ToolCache::new();

        Ok(Self {
            base,
            chain,
            cache,
        })
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.base.start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.base.get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.base.list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.base.delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, AgentError> {
        let conversation = self.base.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ServerError("Conversation not found".to_string()))?;

        // Add system messages based on role if provided
        if let Some(role) = role {
            conversation.add_message("system", role.get_prompt());
            
            if let Some(audience) = role.get_audience() {
                conversation.add_message("system", audience.get_prompt());
            }
            if let Some(preset) = role.get_preset() {
                conversation.add_message("system", preset.get_prompt());
            }
            if let Some(style) = role.get_style() {
                conversation.add_message("system", style.get_prompt());
            }
        }

        // Process content through tool chain if it looks like a web request
        let processed_content = if content.starts_with("http") || content.contains("search:") {
            let tool_input = ToolInput::new(content.to_string());
            let tool_output = self.chain.execute(tool_input).await?;
            tool_output.content
        } else {
            content.to_string()
        };

        conversation.add_message("user", &processed_content);

        let request = ChatRequest {
            model: conversation.model.clone(),
            messages: conversation.messages.iter()
                .map(|m| super::types::Message::from(m.clone()))
                .collect(),
            stream: true,
            temperature: self.base.config.chat.temperature.unwrap_or(0.7),
            max_tokens: self.base.config.chat.max_tokens.unwrap_or(2048) as usize,
        };

        let response = self.base.client
            .post(format!("{}/api/chat", self.base.config.ollama.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AgentError::ServerError(error_text));
        }

        Ok(response)
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<String>, AgentError> {
        let text = String::from_utf8(chunk.to_vec())
            .map_err(|e| AgentError::ServerError(format!("Invalid UTF-8: {}", e)))?;

        let stream_response: StreamResponse = serde_json::from_str(&text)
            .map_err(|e| AgentError::JsonError(e))?;

        if stream_response.done {
            return Ok(None);
        }

        Ok(Some(stream_response.message.content))
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.base.add_message(conversation_id, role, content).await
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn description(&self) -> &str {
        &self.base.description
    }
}

impl ToolingAgent {
    /// Searches the web, because Google is too mainstream.
    pub async fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, AgentError> {
        let tool_input = ToolInput::new(query.to_string());
        let output = self.chain.execute(tool_input).await?;
        let results: Vec<SearchResult> = serde_json::from_str(&output.content)?;
        Ok(results)
    }

    /// Fetches and processes a webpage, because raw HTML is for machines.
    pub async fn fetch_page(&mut self, url: &str) -> Result<ProcessedPage, AgentError> {
        let tool_input = ToolInput::new(url.to_string());
        let output = self.chain.execute(tool_input).await?;
        let page: ProcessedPage = serde_json::from_str(&output.content)?;
        Ok(page)
    }

    /// Collects data from multiple pages, because one page is never enough.
    pub async fn collect_data(&mut self, urls: Vec<String>) -> Result<Vec<ProcessedPage>, AgentError> {
        let mut results = Vec::new();
        for url in urls {
            if let Ok(page) = self.fetch_page(&url).await {
                results.push(page);
            }
        }
        Ok(results)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedPage {
    pub url: String,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
} 