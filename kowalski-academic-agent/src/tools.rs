use kowalski_core::{Tool, ToolInput, ToolOutput, TaskType};
use async_trait::async_trait;
use kowalski_core::config::Config;
use std::fmt;
use serde_json;
use chrono;
use kowalski_core::error::KowalskiError;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

/// Academic-specific task types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcademicTaskType {
    /// Search academic papers
    AcademicSearch,
    /// Generate citations
    CitationGeneration,
    /// Parse academic papers
    PaperParsing,
}

impl fmt::Display for AcademicTaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TaskType for AcademicTaskType {
    fn name(&self) -> &'static str {
        match self {
            Self::AcademicSearch => "academic_search",
            Self::CitationGeneration => "citation_generation",
            Self::PaperParsing => "paper_parsing",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::AcademicSearch => "Search for academic papers across various databases",
            Self::CitationGeneration => "Generate citations in various formats",
            Self::PaperParsing => "Parse and extract information from academic papers",
        }
    }
}

/// A tool for searching academic papers
pub struct AcademicSearchTool {
    config: Config,
}

impl AcademicSearchTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for AcademicSearchTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        Ok(ToolOutput::new(
            json!({
                "results": format!("Academic search results for: {}", input.content),
                "query": input.content
            }),
            Some(json!({
                "search_type": "academic",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        ))
    }
}

/// A tool for generating citations
pub struct CitationGeneratorTool {
    config: Config,
}

impl CitationGeneratorTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for CitationGeneratorTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        Ok(ToolOutput::new(
            json!({
                "citation": format!("Generated citation for: {}", input.content),
                "source": input.content
            }),
            Some(json!({
                "generator_type": "citation",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        ))
    }
}

/// A tool for parsing academic papers
pub struct PaperParserTool {
    config: Config,
}

impl PaperParserTool {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for PaperParserTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String> {
        Ok(ToolOutput::new(
            json!({
                "parsed_content": format!("Parsed paper content: {}", input.content),
                "source": input.content
            }),
            Some(json!({
                "parser_type": "academic_paper",
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_academic_search() {
        let mut search = AcademicSearchTool::new(Config::default());
        let input = "machine learning";
        let result = search.execute(ToolInput::new(
            AcademicTaskType::AcademicSearch.name().to_string(),
            input.to_string(),
            json!({})
        )).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_citation_generator() {
        let mut generator = CitationGeneratorTool::new(Config::default());
        let input = "Smith, J. (2020). Title. Journal.";
        let result = generator.execute(ToolInput::new(
            AcademicTaskType::CitationGeneration.name().to_string(),
            input.to_string(),
            json!({})
        )).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_paper_parser() {
        let mut parser = PaperParserTool::new(Config::default());
        let input = "Abstract: This is a test paper...";
        let result = parser.execute(ToolInput::new(
            AcademicTaskType::PaperParsing.name().to_string(),
            input.to_string(),
            json!({})
        )).await;
        assert!(result.is_ok());
    }
} 