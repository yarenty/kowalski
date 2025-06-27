use crate::config::WebAgentConfig;
use crate::tools::{SearchProvider, SearchTool, WebTaskType};
use async_trait::async_trait;
use kowalski_agent_template::builder::AgentBuilder;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::{Conversation, Message};
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use reqwest::Response;
use serde_json::json;

/// WebAgent: A specialized agent for web-related tasks
/// This agent is built on top of the TemplateAgent and provides web-specific functionality
pub struct WebAgent {
    template: AgentBuilder,
    config: WebAgentConfig,
}

impl WebAgent {
    /// Creates a new WebAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        let web_config = WebAgentConfig::from(config);
        let search_tool = SearchTool::new(web_config.search_provider.clone());

        let builder = GeneralTemplate::create_agent(
            vec![Box::new(search_tool) as Box<dyn Tool + Send + Sync>],
            Some("You are a web research assistant specialized in finding and analyzing online information.".to_string()),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Config(e.to_string()))?;

        Ok(Self {
            template: builder,
            config: web_config,
        })
    }
}

#[async_trait]
impl Agent for WebAgent {
    async fn execute_task(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        self.template.build().await?.execute_task(input).await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.template
            .build()
            .await
            .expect("Failed to build agent")
            .add_message(conversation_id, role, content)
            .await;
    }

    async fn start_conversation(&self, model: &str) -> String {
        self.template
            .build()
            .await
            .expect("Failed to build agent")
            .start_conversation(model)
    }

    fn set_system_prompt(&mut self, prompt: &str) {
        self.template = self.template.with_system_prompt(prompt);
    }

    fn set_temperature(&mut self, temperature: f32) {
        self.template = self.template.with_temperature(temperature);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::error::KowalskiError;

    #[tokio::test]
    async fn test_web_agent_creation() {
        let config = Config::default();
        let agent = WebAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_web_agent_customization() {
        let config = Config::default();
        let mut agent = WebAgent::new(config).await.unwrap();

        agent.set_system_prompt("You are a specialized web research assistant.");
        agent.set_temperature(0.5);
    }
}
