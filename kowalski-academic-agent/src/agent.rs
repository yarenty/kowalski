use crate::config::AcademicAgentConfig;
use crate::create_academic_agent;
use crate::tools::AcademicTaskType;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::{Conversation, Message};
use kowalski_core::error::KowalskiError;
use kowalski_core::role::Role;
use reqwest::Response;
use serde_json::json;

/// AcademicAgent: A specialized agent for academic tasks
/// This agent is built on top of the TemplateAgent and provides academic-specific functionality
pub struct AcademicAgent {
    template: TemplateAgent,
    config: AcademicAgentConfig,
}

impl AcademicAgent {
    /// Creates a new AcademicAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        let template = create_academic_agent(config.clone()).await?;
        let academic_config = AcademicAgentConfig::from(config);
        Ok(Self {
            template,
            config: academic_config,
        })
    }
}

#[async_trait::async_trait]
impl Agent for AcademicAgent {
    async fn new(config: Config) -> Result<Self, KowalskiError>
    where
        Self: Sized,
    {
        AcademicAgent::new(config).await
    }

    fn start_conversation(&mut self, model: &str) -> String {
        self.template.base_mut().start_conversation(model)
    }

    fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.template.base().get_conversation(id)
    }

    fn list_conversations(&self) -> Vec<&Conversation> {
        self.template.base().list_conversations()
    }

    fn delete_conversation(&mut self, id: &str) -> bool {
        self.template.base_mut().delete_conversation(id)
    }

    async fn chat_with_history(
        &mut self,
        conversation_id: &str,
        content: &str,
        role: Option<Role>,
    ) -> Result<Response, KowalskiError> {
        self.template
            .base_mut()
            .chat_with_history(conversation_id, content, role)
            .await
    }

    async fn process_stream_response(
        &mut self,
        conversation_id: &str,
        chunk: &[u8],
    ) -> Result<Option<Message>, KowalskiError> {
        self.template
            .base_mut()
            .process_stream_response(conversation_id, chunk)
            .await
    }

    async fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) {
        self.template
            .base_mut()
            .add_message(conversation_id, role, content)
            .await
    }

    fn name(&self) -> &str {
        self.template.base().name()
    }

    fn description(&self) -> &str {
        self.template.base().description()
    }
}

impl AcademicAgent {
    /// Searches for academic papers
    pub async fn search_papers(&self, query: &str) -> Result<String, KowalskiError> {
        let tool_input = kowalski_core::tools::ToolInput::new(
            AcademicTaskType::AcademicSearch.to_string(),
            query.to_string(),
            json!({}),
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["results"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Generates a citation for a reference
    pub async fn generate_citation(&self, reference: &str) -> Result<String, KowalskiError> {
        let tool_input = kowalski_core::tools::ToolInput::new(
            AcademicTaskType::CitationGeneration.to_string(),
            reference.to_string(),
            json!({}),
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["citation"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Parses and analyzes an academic paper
    pub async fn parse_paper(&self, content: &str) -> Result<String, KowalskiError> {
        let tool_input = kowalski_core::tools::ToolInput::new(
            AcademicTaskType::PaperParsing.to_string(),
            content.to_string(),
            json!({}),
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["parsed_content"]
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

    /// Gets the academic configuration
    pub fn config(&self) -> &AcademicAgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_academic_agent_creation() {
        let config = Config::default();
        let agent = AcademicAgent::new(config);
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_academic_agent_tools() {
        let config = Config::default();
        let agent = AcademicAgent::new(config).unwrap();
        // Test paper search
        let result = agent.search_papers("machine learning").await;
        assert!(result.is_ok());
        // Test citation generation
        let result = agent
            .generate_citation("Smith, J. (2020). Title. Journal.")
            .await;
        assert!(result.is_ok());
        // Test paper parsing
        let result = agent.parse_paper("Abstract: This is a test paper...").await;
        assert!(result.is_ok());
    }
}
