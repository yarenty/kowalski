pub mod agent;
pub mod config;
pub mod error;
pub mod tools;

pub use agent::AcademicAgent;
pub use config::AcademicAgentConfig;

// Re-export common types
pub use kowalski_core::config::Config;
pub use kowalski_core::error::KowalskiError;
pub use kowalski_core::logging;

use crate::tools::{AcademicSearchTool, AcademicTaskType, CitationGeneratorTool, PaperParserTool};
use async_trait::async_trait;
use kowalski_agent_template::agent::TaskHandler;
use kowalski_agent_template::TemplateAgent;
use kowalski_core::tools::{ToolInput, ToolOutput};
use serde_json::json;

/// Creates a new academic agent with the specified configuration
pub async fn create_academic_agent(config: Config) -> Result<TemplateAgent, KowalskiError> {
    let template = TemplateAgent::new(config.clone()).await?;

    // Register tools
    template.register_tool(Box::new(AcademicSearchTool::new(config.clone()))).await;
    template.register_tool(Box::new(CitationGeneratorTool::new(config.clone()))).await;
    template.register_tool(Box::new(PaperParserTool::new(config.clone()))).await;

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
                })),
            ))
        }
    }
    template.register_task_handler(
        AcademicTaskType::AcademicSearch,
        Box::new(AcademicSearchHandler),
    ).await;

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
                })),
            ))
        }
    }
    template.register_task_handler(
        AcademicTaskType::CitationGeneration,
        Box::new(CitationGenerationHandler),
    ).await;

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
                })),
            ))
        }
    }
    template.register_task_handler(
        AcademicTaskType::PaperParsing,
        Box::new(PaperParsingHandler),
    ).await;

    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_academic_agent() {
        let config = Config::default();
        let agent = create_academic_agent(config).await;
        assert!(agent.is_ok());
    }
}
