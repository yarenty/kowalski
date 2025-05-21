use crate::analyzer::CodeAnalyzer;
use crate::config::CodeAgentConfig;
use crate::documentation::CodeDocumenter;
use crate::refactor::CodeRefactorer;
use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{TaskType, Tool, ToolInput, ToolOutput};
use reqwest::Client;
use serde_json::json;
use std::fmt;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

/// Task types specific to code operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeTaskType {
    Analyze,
    Refactor,
    Document,
    Search,
}

impl fmt::Display for CodeTaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TaskType for CodeTaskType {
    fn name(&self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::Refactor => "refactor",
            Self::Document => "document",
            Self::Search => "search",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Analyze => "Analyze code for quality, complexity, and potential issues",
            Self::Refactor => "Refactor code to improve structure and maintainability",
            Self::Document => "Generate documentation for code",
            Self::Search => "Search for code patterns and examples",
        }
    }
}

/// A code analysis tool that performs various types of analysis
pub struct CodeAnalysisTool {
    analyzer: CodeAnalyzer,
    config: CodeAgentConfig,
}

impl CodeAnalysisTool {
    pub fn new(config: CodeAgentConfig) -> Result<Self, KowalskiError> {
        Ok(Self {
            analyzer: CodeAnalyzer::new(config.clone())
                .map_err(|e| KowalskiError::Agent(e.to_string()))?,
            config,
        })
    }
}

#[async_trait]
impl Tool for CodeAnalysisTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        self.analyzer
            .analyze_content(&input.content)
            .map_err(|e| KowalskiError::Agent(e.to_string()))
            .map_err(|e| e.to_string())?;
        let metrics = self.analyzer.metrics().clone();
        Ok(ToolOutput::new(
            json!({
                "metrics": metrics
            }),
            Some(json!({
                "tool": "code_analysis",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            })),
        ))
    }
}

/// A code refactoring tool that performs various refactoring operations
pub struct CodeRefactoringTool {
    refactorer: CodeRefactorer,
    config: CodeAgentConfig,
}

impl CodeRefactoringTool {
    pub fn new(config: CodeAgentConfig) -> Result<Self, KowalskiError> {
        Ok(Self {
            refactorer: CodeRefactorer::new(config.clone())
                .map_err(|e| KowalskiError::Agent(e.to_string()))?,
            config,
        })
    }
}

#[async_trait]
impl Tool for CodeRefactoringTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        self.refactorer
            .refactor_content(&input.content)
            .map_err(|e| KowalskiError::Agent(e.to_string()))
            .map_err(|e| e.to_string())?;
        let changes = self.refactorer.changes().clone();
        Ok(ToolOutput::new(
            json!({
                "changes": changes
            }),
            Some(json!({
                "tool": "code_refactoring",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            })),
        ))
    }
}

/// A code documentation tool that generates various types of documentation
pub struct CodeDocumentationTool {
    documenter: CodeDocumenter,
    config: CodeAgentConfig,
}

impl CodeDocumentationTool {
    pub fn new(config: CodeAgentConfig) -> Result<Self, KowalskiError> {
        Ok(Self {
            documenter: CodeDocumenter::new(config.clone())
                .map_err(|e| KowalskiError::Agent(e.to_string()))?,
            config,
        })
    }
}

#[async_trait]
impl Tool for CodeDocumentationTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        self.documenter
            .document_content(&input.content)
            .map_err(|e| KowalskiError::Agent(e.to_string()))
            .map_err(|e| e.to_string())?;
        let docs = self.documenter.docs().clone();
        Ok(ToolOutput::new(
            json!({
                "docs": docs
            }),
            Some(json!({
                "tool": "code_documentation",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            })),
        ))
    }
}

/// A code search tool that searches for code patterns
pub struct CodeSearchTool {
    client: Client,
    config: CodeAgentConfig,
}

impl CodeSearchTool {
    pub fn new(config: CodeAgentConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.template.request_timeout))
            .user_agent(&config.template.user_agent)
            .pool_max_idle_per_host(config.template.max_concurrent_requests)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }
}

#[async_trait]
impl Tool for CodeSearchTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        // Implement code search logic
        Ok(ToolOutput::new(
            json!({
                "results": format!("Search results for: {}", input.content)
            }),
            Some(json!({
                "tool": "code_search",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            })),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_analysis_tool() {
        let mut tool = CodeAnalysisTool::new(CodeAgentConfig::default()).unwrap();
        let input = ToolInput::new(
            CodeTaskType::Analyze.name().to_string(),
            "fn main() { println!(\"Hello, world!\"); }".to_string(),
            json!({}),
        );
        let result = tool.execute(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_code_refactoring_tool() {
        let mut tool = CodeRefactoringTool::new(CodeAgentConfig::default()).unwrap();
        let input = ToolInput::new(
            CodeTaskType::Refactor.name().to_string(),
            "fn main() { println!(\"Hello, world!\"); }".to_string(),
            json!({}),
        );
        let result = tool.execute(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_code_documentation_tool() {
        let mut tool = CodeDocumentationTool::new(CodeAgentConfig::default()).unwrap();
        let input = ToolInput::new(
            CodeTaskType::Document.name().to_string(),
            "fn main() { println!(\"Hello, world!\"); }".to_string(),
            json!({}),
        );
        let result = tool.execute(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_code_search_tool() {
        let mut tool = CodeSearchTool::new(CodeAgentConfig::default());
        let input = ToolInput::new(
            CodeTaskType::Search.name().to_string(),
            "main function".to_string(),
            json!({}),
        );
        let result = tool.execute(input).await;
        assert!(result.is_ok());
    }
}
