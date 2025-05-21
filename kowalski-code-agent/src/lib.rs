pub mod agent;
pub mod analyzer;
pub mod config;
pub mod documentation;
pub mod error;
pub mod parser;
pub mod refactor;
pub mod tools;

pub use agent::CodeAgent;
pub use config::CodeAgentConfig;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;

use crate::tools::{
    CodeAnalysisTool, CodeDocumentationTool, CodeRefactoringTool, CodeSearchTool, CodeTaskType,
};
use async_trait::async_trait;
use kowalski_agent_template::agent::TaskHandler;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::tools::{ToolInput, ToolOutput};
use serde_json::json;

/// Creates a new code agent with the specified configuration
pub async fn create_code_agent(config: Config) -> Result<TemplateAgent, KowalskiError> {
    let mut template = TemplateAgent::new(config.clone())?;
    let code_config = CodeAgentConfig::from(config);

    // Configure system prompt
    template = template.with_system_prompt(
        "You are a code-savvy assistant that helps users analyze, refactor, and document code.",
    );

    // Register code-specific tools
    let analysis_tool = Box::new(CodeAnalysisTool::new(code_config.clone())?);
    template.register_tool(analysis_tool).await;

    let refactoring_tool = Box::new(CodeRefactoringTool::new(code_config.clone())?);
    template.register_tool(refactoring_tool).await;

    let documentation_tool = Box::new(CodeDocumentationTool::new(code_config.clone())?);
    template.register_tool(documentation_tool).await;

    let search_tool = Box::new(CodeSearchTool::new(code_config.clone()));
    template.register_tool(search_tool).await;

    // Register task handlers
    struct AnalyzeHandler;
    #[async_trait]
    impl TaskHandler for AnalyzeHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Analyzing code: {}", input.content)
                }),
                Some(json!({
                    "handler": "analyze",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template.register_task_handler(CodeTaskType::Analyze, Box::new(AnalyzeHandler)).await;

    struct RefactorHandler;
    #[async_trait]
    impl TaskHandler for RefactorHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Refactoring code: {}", input.content)
                }),
                Some(json!({
                    "handler": "refactor",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template.register_task_handler(CodeTaskType::Refactor, Box::new(RefactorHandler)).await;

    struct DocumentHandler;
    #[async_trait]
    impl TaskHandler for DocumentHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Documenting code: {}", input.content)
                }),
                Some(json!({
                    "handler": "document",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template.register_task_handler(CodeTaskType::Document, Box::new(DocumentHandler)).await;

    struct SearchHandler;
    #[async_trait]
    impl TaskHandler for SearchHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Searching code: {}", input.content)
                }),
                Some(json!({
                    "handler": "search",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                })),
            ))
        }
    }
    template.register_task_handler(CodeTaskType::Search, Box::new(SearchHandler)).await;

    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_code_agent() {
        let config = Config::default();
        let agent = create_code_agent(config).await;
        assert!(agent.is_ok());
    }
}
