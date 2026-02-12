use crate::config::WebAgentConfig;
use async_trait::async_trait;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Conversation;
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use kowalski_core::template::TemplateAgent;
use kowalski_core::template::default::DefaultTemplate;
use kowalski_core::tools::{Tool, ToolOutput};
use kowalski_tools::web::{WebScrapeTool, WebSearchTool};

use serde::{Deserialize, Serialize};

/// WebAgent: A specialized agent for web-related tasks
/// This agent is built on top of the TemplateAgent and provides web-specific functionality
#[allow(dead_code)]
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

        // Create the agent first without a system prompt
        let builder = DefaultTemplate::create_agent(tools, None, Some(0.7))
            .await
            .map_err(|e| KowalskiError::Configuration(e.to_string()))?;
        let mut agent = builder.build().await?;

        // Generate tool descriptions dynamically from the agent's ToolManager
        let available_tools_section = agent.base().tool_manager.generate_tool_descriptions().await;

        let system_prompt = format!(
            r#"You are a web research assistant specialized in finding and analyzing online information. 

{}

TOOL USAGE INSTRUCTIONS:
- ALWAYS use tools when asked about current events, recent information, or anything that requires up-to-date data
- For general queries, start with web_search to find relevant URLs
- When you have a specific URL, use web_scrape to get detailed content
- You can chain tools: search first, then scrape interesting URLs

RESPONSE FORMAT:
When you need to use a tool, respond with JSON in this exact format:
{{
  "name": "web_search",
  "parameters": {{ "query": "your search query here" }},
  "reasoning": "why you're using this tool"
}}

When you have a final answer, respond normally without JSON formatting.

Remember: Use tools proactively to provide accurate, up-to-date information!"#,
            available_tools_section
        );
        let system_prompt_clone = system_prompt.clone();

        // Ensure the system prompt is set on the base agent
        agent.base_mut().set_system_prompt(&system_prompt_clone);
        Ok(Self {
            agent,
            config: web_config,
        })
    }

    pub async fn list_tools(&self) -> Vec<(String, String)> {
        self.agent.list_tools().await
    }
}

#[async_trait]
impl Agent for WebAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError> {
        WebAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        let system_prompt = {
            let base = self.agent.base();
            base.system_prompt
                .as_deref()
                .unwrap_or("You are a helpful assistant.")
                .to_string()
        };
        let conv_id = self.agent.base_mut().start_conversation(model);
        if let Some(conversation) = self.agent.base_mut().conversations.get_mut(&conv_id) {
            conversation.add_message("system", &system_prompt);
        }
        conv_id
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
    ) -> Result<String, KowalskiError> {
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
            .await
    }

    async fn execute_tool(
        &mut self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Result<ToolOutput, KowalskiError> {
        self.agent.execute_tool(tool_name, tool_input).await
    }

    async fn list_tools(&self) -> Vec<(String, String)> {
        self.agent.list_tools().await
    }

    fn export_conversation(&self, id: &str) -> Result<String, KowalskiError> {
        self.agent.base().export_conversation(id)
    }

    fn import_conversation(&mut self, json_str: &str) -> Result<String, KowalskiError> {
        self.agent.base_mut().import_conversation(json_str)
    }

    fn name(&self) -> &str {
        "Web Agent"
    }

    fn description(&self) -> &str {
        "A specialized agent for web-based tasks like searching and scraping"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
