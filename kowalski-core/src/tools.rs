use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Trait for task types that can be executed by tools
pub trait TaskType: Send + Sync + Display + 'static {
    /// Get the name of the task type
    fn name(&self) -> &str;
    
    /// Get the description of the task type
    fn description(&self) -> &str;
}

/// A tool that can be executed by the agent
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Execute the tool with the given input
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, String>;
}

/// Input for a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// The task type to execute
    pub task_type: String,
    /// The content to process
    pub content: String,
    /// The input parameters for the task
    pub parameters: serde_json::Value,
}

impl ToolInput {
    pub fn new(task_type: String, content: String, parameters: serde_json::Value) -> Self {
        Self { task_type, content, parameters }
    }
}

/// Output from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// The result of the tool execution
    pub result: serde_json::Value,
    /// Any metadata about the execution
    pub metadata: Option<serde_json::Value>,
}

impl ToolOutput {
    pub fn new(result: serde_json::Value, metadata: Option<serde_json::Value>) -> Self {
        Self { result, metadata }
    }
} 