use async_trait::async_trait;
use reqwest::Response;
use crate::config::Config;
use crate::conversation::Conversation;
use crate::role::Role;
use super::{Agent, AgentError, BaseAgent};
use crate::tools::{ToolChain, ToolType, SearchTool, TaskType};
use crate::tools::search::SearchProvider;
use crate::tools::browser::WebBrowser;
use crate::tools::scraper::WebScraper;
use log::{debug, info};
use std::collections::HashMap;

/// UnifiedAgent: A versatile agent that combines chat capabilities with tool usage
pub struct UnifiedAgent {
    base: BaseAgent,
    tool_chain: ToolChain,
}

#[async_trait]
impl Agent for UnifiedAgent {
    fn new(config: Config) -> Result<Self, AgentError> {
        let base = BaseAgent::new(
            config.clone(),
            "UnifiedAgent",
            "A versatile agent that combines chat capabilities with dynamic tool usage",
        )?;

        let mut tool_chain = ToolChain::new();
        
        // Initialize tools
        tool_chain.add_tool(
            ToolType::Search(SearchTool::new(
                SearchProvider::DuckDuckGo,
                config.search.api_key.unwrap_or_default(),
            )),
            vec![TaskType::Search]
        );
        
        tool_chain.add_tool(
            ToolType::Browser(WebBrowser::new(config.clone())),
            vec![TaskType::BrowseDynamic]
        );
        
        tool_chain.add_tool(
            ToolType::Scraper(WebScraper::new()),
            vec![TaskType::ScrapStatic]
        );

        Ok(Self { base, tool_chain })
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
        debug!("Processing chat request: {}", content);

        // Process content through tool chain if needed
        let processed_content = if self.should_use_tools(content) {
            info!("Using tools to process: {}", content);
            let tool_input = crate::tools::ToolInput::new(content.to_string());
            let tool_output = self.tool_chain.execute(tool_input).await?;
            tool_output.content
        } else {
            content.to_string()
        };

        let conversation = self.base.conversations.get_mut(conversation_id)
            .ok_or_else(|| AgentError::ConversationNotFound(conversation_id.to_string()))?;

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

        conversation.add_message("user", &processed_content);

        let request = super::types::ChatRequest {
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
        self.base.process_stream_response(conversation_id, chunk).await
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

impl UnifiedAgent {
    /// Determines if the input requires tool usage
    fn should_use_tools(&self, content: &str) -> bool {
        content.starts_with("http") || 
        content.contains("search:") || 
        content.contains("browse:") || 
        content.contains("scrape:")
    }

    /// Execute a specific tool directly
    pub async fn execute_tool(&self, tool_type: TaskType, query: &str) -> Result<String, AgentError> {
        let tool_input = crate::tools::ToolInput::new(query.to_string());
        let tool_output = self.tool_chain.execute(tool_input).await?;
        Ok(tool_output.content)
    }

    /// Configure tool-specific options
    pub fn configure_tool(&mut self, tool_type: TaskType, options: HashMap<String, String>) {
        // TODO: Implement tool configuration
        debug!("Configuring tool {:?} with options: {:?}", tool_type, options);
    }
}