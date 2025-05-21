/// Tools module: Because every AI needs its gadgets.
/// "Tools are like toys for grown-up developers." - A Tool Enthusiast
mod browser;
mod cache;
mod scraper;
mod search;
mod chain;
mod web;

pub use crate::utils::KowalskiError;
pub use browser::WebBrowser;
pub use cache::ToolCache;
pub use scraper::WebScraper;
pub use search::{SearchProvider, SearchTool};
pub use chain::ToolChain;
pub use web::{WebBrowser, WebScraper};

use async_trait::async_trait;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

/// The core trait for all tools, because every tool needs a purpose.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}

/// Input for tools, because tools need something to work with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub query: String,
    pub parameters: HashMap<String, Value>,
}

impl fmt::Display for ToolInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.query)
    }
}

impl ToolInput {
    pub fn new(query: String) -> Self {
        Self {
            query,
            parameters: HashMap::new(),
        }
    }
}

/// Output from tools, because tools need to show off their work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub metadata: HashMap<String, Value>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Search,
    BrowseDynamic,
    ScrapStatic,
}

pub struct TaskRouter {
    patterns: HashMap<String, TaskType>,
}

impl TaskRouter {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Search patterns
        patterns.insert("search:".to_string(), TaskType::Search);
        patterns.insert("find:".to_string(), TaskType::Search);
        patterns.insert("lookup:".to_string(), TaskType::Search);

        // Dynamic content patterns
        patterns.insert("twitter.com".to_string(), TaskType::BrowseDynamic);
        patterns.insert("linkedin.com".to_string(), TaskType::BrowseDynamic);
        patterns.insert("facebook.com".to_string(), TaskType::BrowseDynamic);

        // Static content patterns (default for most URLs)
        patterns.insert("github.com".to_string(), TaskType::ScrapStatic);
        patterns.insert("docs.rs".to_string(), TaskType::ScrapStatic);

        Self { patterns }
    }

    pub fn determine_task_type(&self, input: &str) -> TaskType {
        // Check for explicit search queries
        if input.starts_with("search:")
            || input.starts_with("find:")
            || input.starts_with("lookup:")
        {
            return TaskType::Search;
        }

        // Check URL patterns
        if input.starts_with("http") || input.starts_with("https") {
            for (pattern, task_type) in &self.patterns {
                if input.contains(pattern) {
                    return task_type.clone();
                }
            }
            // Default to static scraping for unknown URLs
            return TaskType::ScrapStatic;
        }

        TaskType::Unknown
    }
}

#[derive(Debug, Clone)]
pub enum ToolType {
    Browser(browser::WebBrowser),
    Search(search::SearchTool),
    Scraper(scraper::WebScraper),
}

impl ToolType {
    fn name(&self) -> &str {
        match self {
            ToolType::Browser(b) => b.name(),
            ToolType::Search(s) => s.name(),
            ToolType::Scraper(s) => s.name(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        match self {
            ToolType::Browser(b) => b.execute(input).await,
            ToolType::Search(s) => s.execute(input).await,
            ToolType::Scraper(s) => s.execute(input).await,
        }
    }
}

pub struct ToolChain {
    tools: HashMap<TaskType, Vec<ToolType>>,
    router: TaskRouter,
}

impl ToolChain {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            router: TaskRouter::new(),
        }
    }

    pub fn add_tool(&mut self, tool: ToolType, task_types: Vec<TaskType>) {
        for task_type in task_types {
            self.tools
                .entry(task_type)
                .or_insert_with(Vec::new)
                .push(tool.clone());
        }
    }

    pub async fn execute(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let task_type = self.router.determine_task_type(&input.query);

        if let Some(tools) = self.tools.get(&task_type) {
            if let Some(tool) = tools.first() {
                debug!(
                    "[{}:{}] Using tool {} for task type {:?}",
                    file!(),
                    line!(),
                    tool.name(),
                    task_type
                );
                return tool.execute(input).await;
            }
        }

        Err(KowalskiError::NoSuitableTool(format!(
            "No suitable tool found for task type {:?}",
            task_type
        )))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("No tool available: {0}")]
    NoToolAvailable(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_input_creation() {
        let input = ToolInput::new("test query".to_string());
        assert_eq!(input.query, "test query");
        assert!(input.parameters.is_empty());
    }
}
