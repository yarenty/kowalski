use crate::config::WebAgentConfig;
use async_trait::async_trait;
use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_tools::web::{WebScrapeTool, WebSearchTool};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// WebAgent: A specialized agent for web-related tasks
/// This agent is built on top of the TemplateAgent and provides web-specific functionality
pub struct WebAgent {
    agent: TemplateAgent,
    config: WebAgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult {
    pub title: String,
    pub content: String,
}

impl WebAgent {
    /// Creates a new WebAgent with the specified configuration
    pub async fn new(_config: Config) -> Result<Self, KowalskiError> {
        // TODO: Convert Config to WebAgentConfig if needed
        let web_config = WebAgentConfig::default();
        let provider = web_config.search.default_provider.to_string();
        let search_tool = WebSearchTool::new(provider);
        let scrape_tool = WebScrapeTool::new();
        let tools: Vec<Box<dyn Tool + Send + Sync>> =
            vec![Box::new(search_tool), Box::new(scrape_tool)];
        let builder = GeneralTemplate::create_agent(
            tools,
            Some("You are a web research assistant specialized in finding and analyzing online information.".to_string()),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let agent = builder.build().await?;
        Ok(Self {
            agent,
            config: web_config,
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, KowalskiError> {
        use serde_json::Value;
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "web_search");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "web_search tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "search".to_string(),
            query.to_string(),
            serde_json::json!({"query": query}),
        );
        let output = tool.execute(input).await?;
        // Try to parse the results field as a list of SearchResult
        let results_val = &output.result["results"];
        // If it's a string (e.g. raw JSON from DuckDuckGo), try to parse it
        let results: Vec<SearchResult> = if results_val.is_string() {
            let raw = results_val.as_str().unwrap();
            // Try to parse as JSON array, fallback to empty
            serde_json::from_str(raw).unwrap_or_default()
        } else if results_val.is_array() {
            serde_json::from_value(results_val.clone()).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(results)
    }

    pub async fn fetch_page(&self, url: &str) -> Result<PageResult, KowalskiError> {
        use serde_json::Value;
        let mut tools = self.agent.tool_chain.write().await;
        let tool = tools.iter_mut().find(|t| t.name() == "web_scrape");
        let tool = match tool {
            Some(t) => t,
            None => {
                return Err(KowalskiError::ToolExecution(
                    "web_scrape tool not found".to_string(),
                ));
            }
        };
        let input = ToolInput::new(
            "scrape_static".to_string(),
            url.to_string(),
            serde_json::json!({"url": url, "selectors": ["title", "body"]}),
        );
        let output = tool.execute(input).await?;
        // Parse the first result as the page title/content
        let arr = output.result.as_array().cloned().unwrap_or_default();
        let title = arr
            .iter()
            .find(|v| v["selector"] == "title")
            .and_then(|v| v["text"].as_str())
            .unwrap_or("")
            .to_string();
        let content = arr
            .iter()
            .find(|v| v["selector"] == "body")
            .and_then(|v| v["text"].as_str())
            .unwrap_or("")
            .to_string();
        Ok(PageResult { title, content })
    }
}

#[async_trait]
impl Agent for WebAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        WebAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.agent.base_mut().start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.agent.base().get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.agent.base().list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.agent.base_mut().delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<reqwest::Response, KowalskiError> {
        self.agent
            .base_mut()
            .chat_with_history(conversation_id, content, role)
            .await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<kowalski_core::conversation::Message>, KowalskiError> {
        self.agent
            .base_mut()
            .process_stream_response(conversation_id, chunk)
            .await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.agent
            .base_mut()
            .add_message(conversation_id, role, content)
            .await;
    }

    fn name(&self) -> &str {
        self.agent.base().name()
    }

    fn description(&self) -> &str {
        self.agent.base().description()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::config::Config;

    #[tokio::test]
    async fn test_web_agent_creation() {
        let config = Config::default();
        let agent = WebAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_web_agent_conversation() {
        let config = Config::default();
        let mut agent = WebAgent::new(config).await.unwrap();
        let conv_id = agent.start_conversation("test-model");
        assert!(!conv_id.is_empty());
    }
}
