/// Tools module: Because every AI needs its gadgets.
/// "Tools are like toys for grown-up developers." - A Tool Enthusiast

mod browser;
mod search;
mod scraper;
mod cache;
mod error;

pub use browser::WebBrowser;
pub use search::SearchTool;
pub use scraper::WebScraper;
pub use cache::{ToolCache, Storage};
pub use error::ToolError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

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
    pub parameters: HashMap<String, serde_json::Value>,
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

impl ToolChain {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            cache: ToolCache::new(),
        }
    }

    pub fn add<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        if let Some(cached) = self.cache.get(&input) {
            return Ok(cached);
        }

        let mut current_input = input;
        let mut final_output = None;

        for tool in &self.tools {
            let output = tool.execute(current_input).await?;
            self.cache.set(&current_input, &output);
            
            current_input = ToolInput {
                query: output.content.clone(),
                parameters: output.metadata.clone(),
            };
            
            final_output = Some(output);
        }

        final_output.ok_or(ToolError::NoOutput)
    }
}