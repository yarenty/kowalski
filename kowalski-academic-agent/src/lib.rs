pub mod agent;
pub mod tools;
pub mod error;
pub mod config;

pub use agent::AcademicAgent;
pub use config::AcademicAgentConfig;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;

use kowalski_agent_template::TemplateAgent;
use kowalski_agent_template::agent::TaskHandler;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput, TaskType};
use crate::tools::{AcademicTaskType, AcademicSearchTool, CitationGeneratorTool, PaperParserTool};
use serde_json::json;
use async_trait::async_trait;

/// Creates a new academic agent with the specified configuration
pub fn create_academic_agent(config: Config) -> Result<TemplateAgent, KowalskiError> {
    let mut template = TemplateAgent::new(config.clone())?;

    // Register tools
    template.register_tool(Box::new(AcademicSearchTool::new(config.clone())));
    template.register_tool(Box::new(CitationGeneratorTool::new(config.clone())));
    template.register_tool(Box::new(PaperParserTool::new(config.clone())));

    // Register task handlers
    struct AcademicSearchHandler;
    #[async_trait]
    impl TaskHandler for AcademicSearchHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Searching academic papers: {}", input.content)
                }),
                Some(json!({
                    "handler": "academic_search",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                }))
            ))
        }
    }
    template.register_task_handler(AcademicTaskType::AcademicSearch, Box::new(AcademicSearchHandler));

    struct CitationGenerationHandler;
    #[async_trait]
    impl TaskHandler for CitationGenerationHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Generating citation: {}", input.content)
                }),
                Some(json!({
                    "handler": "citation_generation",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                }))
            ))
        }
    }
    template.register_task_handler(AcademicTaskType::CitationGeneration, Box::new(CitationGenerationHandler));

    struct PaperParsingHandler;
    #[async_trait]
    impl TaskHandler for PaperParsingHandler {
        async fn handle(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
            Ok(ToolOutput::new(
                json!({
                    "result": format!("Parsing paper: {}", input.content)
                }),
                Some(json!({
                    "handler": "paper_parsing",
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                }))
            ))
        }
    }
    template.register_task_handler(AcademicTaskType::PaperParsing, Box::new(PaperParsingHandler));

    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_academic_agent() {
        let config = Config::default();
        let agent = create_academic_agent(config);
        assert!(agent.is_ok());
    }
} 