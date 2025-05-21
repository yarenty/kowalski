use kowalski_agent_template::TemplateAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput, TaskType};
use serde_json::json;
use crate::tools::{CodeTaskType, CodeAnalysisTool, CodeRefactoringTool, CodeDocumentationTool, CodeSearchTool};
use crate::config::CodeAgentConfig;

/// CodeAgent: A specialized agent for code analysis and processing
pub struct CodeAgent {
    template: TemplateAgent,
    config: CodeAgentConfig,
}

impl CodeAgent {
    /// Creates a new CodeAgent with the specified configuration
    pub fn new(config: Config) -> Result<Self, KowalskiError> {
        let mut template = TemplateAgent::new(config.clone())?;
        let code_config = CodeAgentConfig::from(config);

        // Configure system prompt
        template = template.with_system_prompt(
            "You are a code-savvy assistant that helps users analyze, refactor, and document code.",
        );

        // Register code-specific tools
        let analysis_tool = Box::new(CodeAnalysisTool::new(code_config.clone())?);
        template.register_tool(analysis_tool);

        let refactoring_tool = Box::new(CodeRefactoringTool::new(code_config.clone())?);
        template.register_tool(refactoring_tool);

        let documentation_tool = Box::new(CodeDocumentationTool::new(code_config.clone())?);
        template.register_tool(documentation_tool);

        let search_tool = Box::new(CodeSearchTool::new(code_config.clone()));
        template.register_tool(search_tool);

        Ok(Self { template, config: code_config })
    }

    /// Analyzes code using the code analysis tool
    pub async fn analyze_code(&self, code: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            CodeTaskType::Analyze.name().to_string(),
            code.to_string(),
            json!({})
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["metrics"].as_str().unwrap_or_default().to_string())
    }

    /// Refactors code using the code refactoring tool
    pub async fn refactor_code(&self, code: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            CodeTaskType::Refactor.name().to_string(),
            code.to_string(),
            json!({})
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["changes"].as_str().unwrap_or_default().to_string())
    }

    /// Generates documentation using the code documentation tool
    pub async fn document_code(&self, code: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            CodeTaskType::Document.name().to_string(),
            code.to_string(),
            json!({})
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["docs"].as_str().unwrap_or_default().to_string())
    }

    /// Searches for code patterns using the code search tool
    pub async fn search_code(&self, query: &str) -> Result<String, KowalskiError> {
        let tool_input = ToolInput::new(
            CodeTaskType::Search.name().to_string(),
            query.to_string(),
            json!({})
        );
        let tool_output = self.template.execute_task(tool_input).await?;
        Ok(tool_output.result["results"].as_str().unwrap_or_default().to_string())
    }

    /// Gets the underlying template agent
    pub fn template(&self) -> &TemplateAgent {
        &self.template
    }

    /// Gets a mutable reference to the underlying template agent
    pub fn template_mut(&mut self) -> &mut TemplateAgent {
        &mut self.template
    }

    /// Gets the code agent configuration
    pub fn config(&self) -> &CodeAgentConfig {
        &self.config
    }

    /// Gets a mutable reference to the code agent configuration
    pub fn config_mut(&mut self) -> &mut CodeAgentConfig {
        &mut self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_agent_creation() {
        let config = Config::default();
        let agent = CodeAgent::new(config);
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_code_agent_tools() {
        let config = Config::default();
        let agent = CodeAgent::new(config).unwrap();
        
        // Test code analysis
        let result = agent.analyze_code("fn main() { println!(\"Hello, world!\"); }").await;
        assert!(result.is_ok());
        
        // Test code refactoring
        let result = agent.refactor_code("fn main() { println!(\"Hello, world!\"); }").await;
        assert!(result.is_ok());
        
        // Test code documentation
        let result = agent.document_code("fn main() { println!(\"Hello, world!\"); }").await;
        assert!(result.is_ok());
        
        // Test code search
        let result = agent.search_code("main function").await;
        assert!(result.is_ok());
    }
} 