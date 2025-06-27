use crate::create_web_agent;
use crate::tools::WebTaskType;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{TaskType, ToolInput};
use kowalski_core::Agent;
use serde_json::json;
use kowalski_core::conversation::Conversation;
use kowalski_core::conversation::Message;
use kowalski_core::role::Role;
use reqwest::Response;
use async_trait::async_trait;

/// WebAgent: A specialized agent for web-related tasks
/// This agent is built on top of the TemplateAgent and provides web-specific functionality
pub struct WebAgent {
    template: TemplateAgent,
}

#[async_trait]
impl Agent for WebAgent {

    async fn new(config:Config) -> Result<Self,KowalskiError>where Self:Sized {
        let agent = TemplateAgent::new(config)?;
     Ok(Self{
        template: agent
     })
    }

    fn start_conversation(&mut self,model: &str) -> String {
        self.template.base_mut().start_conversation(model)
    }


    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.template.base().get_conversation(id)
    }

    /// Lists all conversations
    fn list_conversations(&self) -> Vec<&Conversation>{
        self.template.base().list_conversations()
    }

    /// Deletes a conversation
    fn delete_conversation(&mut self, id: &str) -> bool {
        self.template.base_mut().delete_conversation(id)

    }

    /// Chats with history
    /// 
    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError> {
        self.template.base_mut().chat_with_history(conversation_id, content, role).await

    }

    /// Processes a stream response
    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError> {
        self.template.base_mut().process_stream_response(conversation_id, chunk).await

    }

    /// Adds a message to a conversation
    async fn add_message(
        &mut self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) {
        self.template.base_mut().add_message(conversation_id, role, content).await
    }

    /// Gets the agent's name
    fn name(&self) -> &str {
        "WebAgent"
    }

    /// Gets the agent's description
    fn description(&self) -> &str {
        "Agent to connect to Web"
    }

}



impl WebAgent {
    /// Creates a new WebAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        let template = create_web_agent(config).await?;
        Ok(Self { template })
    }

    /// Searches the web using the configured search tool
    pub async fn search(&self, query: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            WebTaskType::Search.name().to_string(),
            query.to_string(),
            json!({}),
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["results"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Fetches and processes a webpage
    pub async fn fetch_page(&self, url: &str) -> Result<String, KowalskiError> {
        let task_type = if url.contains("twitter.com")
            || url.contains("linkedin.com")
            || url.contains("facebook.com")
        {
            WebTaskType::BrowseDynamic
        } else {
            WebTaskType::ScrapeStatic
        };

        let tool_input = ToolInput::new(task_type.name().to_string(), url.to_string(), json!({}));
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["content"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Gets the underlying template agent
    pub fn template(&self) -> &TemplateAgent {
        &self.template
    }

    /// Gets a mutable reference to the underlying template agent
    pub fn template_mut(&mut self) -> &mut TemplateAgent {
        &mut self.template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_agent_creation() {
        let config = Config::default();
        let agent = WebAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_web_agent_tools() {
        let config = Config::default();
        let agent = WebAgent::new(config).await.unwrap();

        // Test search
        let result = agent.search("test query").await;
        assert!(result.is_ok());

        // Test page fetch
        let result = agent.fetch_page("https://example.com").await;
        assert!(result.is_ok());
    }
}
