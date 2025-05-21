use crate::config::AcademicAgentConfig;
use crate::create_academic_agent;
use crate::tools::AcademicTaskType;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
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
