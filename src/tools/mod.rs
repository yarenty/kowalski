/// Tools module: Because every AI needs its gadgets.
/// "Tools are like toys for grown-up developers." - A Tool Enthusiast

mod browser;
mod scraper;
mod search;
mod cache;

pub use browser::WebBrowser;
pub use scraper::WebScraper;
pub use search::{SearchTool, SearchProvider};
pub use cache::ToolCache;
pub use crate::utils::KowalskiError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use log::debug;

/// The core trait for all tools, because every tool needs a purpose.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    #[allow(dead_code)]
    fn description(&self) -> &str;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError>;
}

/// Input for tools, because tools need something to work with.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ToolInput {
    pub query: String,
    pub context: Option<String>,
}

impl fmt::Display for ToolInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.query, self.context.as_deref().unwrap_or(""))
    }
}

#[allow(dead_code)]
impl ToolInput {
    pub fn new(query: String) -> Self {
        Self {
            query,
            context: None,
        }
    }

    pub fn with_context(query: String, context: String) -> Self {
        Self {
            query,
            context: Some(context),
        }
    }
}

/// Output from tools, because tools need to show off their work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum TaskType {
    Search,           // General web search
    BrowseDynamic,   // JavaScript-heavy sites needing browser
    ScrapStatic,     // Static HTML content
    Unknown,
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
        if input.starts_with("search:") || input.starts_with("find:") || input.starts_with("lookup:") {
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

#[derive(Clone)]
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
            self.tools.entry(task_type)
                .or_insert_with(Vec::new)
                .push(tool.clone());
        }
    }

    pub async fn execute(&self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let task_type = self.router.determine_task_type(&input.query);
        
        if let Some(tools) = self.tools.get(&task_type) {
            if let Some(tool) = tools.first() {
                debug!("[{}:{}] Using tool {} for task type {:?}", 
                    file!(), line!(), tool.name(), task_type);
                return tool.execute(input).await;
            }
        }
        
        Err(KowalskiError::NoSuitableTool(format!(
            "No suitable tool found for task type {:?}", task_type
        )))
    }
}