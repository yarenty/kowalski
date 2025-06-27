use crate::config::CodeAgentConfig;
use crate::tools::{CodeAnalysisTool, CodeRefactoringTool, CodeDocumentationTool, CodeSearchTool};
use kowalski_agent_template::builder::AgentBuilder;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use serde_json::json;
use kowalski_core::conversation::{Conversation, Message};
use kowalski_core::role::Role;
use async_trait::async_trait;

/// CodeAgent: A specialized agent for code analysis and development tasks
/// This agent is built on top of the TemplateAgent and provides code-specific functionality
pub struct CodeAgent {
    template: AgentBuilder,
    config: CodeAgentConfig,
}

impl CodeAgent {
    /// Creates a new CodeAgent with the specified configuration
    pub async fn new(config: Config) -> Result<Self, KowalskiError> {
        let code_config = CodeAgentConfig::from(config);
        
        // Create code-specific tools
        let analysis_tool = CodeAnalysisTool::new(code_config.clone())?;
        let refactoring_tool = CodeRefactoringTool::new(code_config.clone())?;
        let documentation_tool = CodeDocumentationTool::new(code_config.clone())?;
        let search_tool = CodeSearchTool::new(code_config.clone());

        let tools = vec![
            Box::new(analysis_tool) as Box<dyn Tool + Send + Sync>,
            Box::new(refactoring_tool) as Box<dyn Tool + Send + Sync>,
            Box::new(documentation_tool) as Box<dyn Tool + Send + Sync>,
            Box::new(search_tool) as Box<dyn Tool + Send + Sync>,
        ];

        let builder = GeneralTemplate::create_agent(
            tools,
            Some("You are a code analysis and development assistant specialized in analyzing, refactoring, and documenting code.".to_string()),
            Some(0.7),
        )
        .await
        .map_err(|e| KowalskiError::Config(e.to_string()))?;

        Ok(Self {
            template: builder,
            config: code_config,
        })
    }

    /// Analyzes code
    pub async fn analyze_code(&self, code: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            "code_analysis".to_string(),
            code.to_string(),
            json!({
                "language": self.config.language,
                "max_depth": self.config.max_analysis_depth
            }),
        );
        let tool_output = self.template.build().await?.execute_task(tool_input).await?;
        Ok(tool_output.result["analysis"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Refactors code
    pub async fn refactor_code(&self, code: &str, description: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            "code_refactoring".to_string(),
            code.to_string(),
            json!({
                "description": description,
                "language": self.config.language
            }),
        );
        let tool_output = self.template.build().await?.execute_task(tool_input).await?;
        Ok(tool_output.result["refactored_code"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Generates documentation
    pub async fn generate_docs(&self, code: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            "code_documentation".to_string(),
            code.to_string(),
            json!({
                "language": self.config.language,
                "style": self.config.documentation_style
            }),
        );
        let tool_output = self.template.build().await?.execute_task(tool_input).await?;
        Ok(tool_output.result["documentation"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Searches codebase
    pub async fn search_code(&self, query: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            "code_search".to_string(),
            query.to_string(),
            json!({
                "language": self.config.language,
                "scope": self.config.search_scope
            }),
        );
        let tool_output = self.template.build().await?.execute_task(tool_input).await?;
        Ok(tool_output.result["results"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }
}

#[async_trait]
impl Agent for CodeAgent {
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
    async fn test_code_agent_creation() {
        let config = Config::default();
        let agent = CodeAgent::new(config).await;
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_code_agent_customization() {
        let config = Config::default();
        let mut agent = CodeAgent::new(config).await.unwrap();
        
        agent.set_system_prompt("You are a specialized code development assistant.");
        agent.set_temperature(0.5);
    }
}
