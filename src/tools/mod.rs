/// Tools module: Because every AI needs its gadgets.
/// "Tools are like toys for grown-up developers." - A Tool Enthusiast

mod browser;
pub mod search;
mod scraper;
mod cache;
mod error;

// pub use browser::WebBrowser;
pub use search::SearchTool;
// pub use scraper::WebScraper;
pub use cache::ToolCache;
pub use error::ToolError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use log::{debug, info};

/// The core trait for all tools, because every tool needs a purpose.
#[async_trait]
#[allow(dead_code)]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
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

/// Chain of tools, because one tool is never enough.
pub struct ToolChain {
    tools: Vec<Box<dyn Tool>>,
    cache: ToolCache,
}

#[allow(dead_code)]
impl ToolChain {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            cache: ToolCache::new(),
        }
    }

    pub fn add_tool<T: Tool + 'static>(&mut self, tool: Box<T>) {
        self.tools.push(tool);
    }

    pub fn add<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        debug!("Executing tool chain with input: {}", input);
        if let Some(cached) = self.cache.get(&input) {
            debug!("Cache hit for input: {}", input);
            return Ok(cached);
        }

        let mut current_input = input.clone();
        let mut final_output = None;

        info!("Tools in chain: {}", self.tools.iter().map(|t| t.name()).collect::<Vec<_>>().join(", "));

        for tool in &self.tools {
            debug!("Executing tool: {}", tool.name());
            let output = tool.execute(current_input).await?;
            debug!("Tool output: {:?}", output);
            self.cache.set(&input, &output);

            debug!("Cache set for input: {}", input);
            
            current_input = ToolInput::new(output.content.clone());
            final_output = Some(output);
        }

        final_output.ok_or(ToolError::NoOutput)
    }
}