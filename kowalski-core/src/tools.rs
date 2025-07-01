use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub parameter_type: ParameterType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// Trait for task types that can be executed by tools
pub trait TaskType: Send + Sync + Display {
    /// Get the name of the task type
    fn name(&self) -> &str;

    /// Get the description of the task type
    fn description(&self) -> &str;
}

/// A tool that can be executed by the agent
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Execute the tool with the given input
    async fn execute(
        &mut self,
        input: ToolInput,
    ) -> Result<ToolOutput, crate::error::KowalskiError>;

    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Vec<ToolParameter>;

    fn validate_input(&self, input: &ToolInput) -> Result<(), crate::error::KowalskiError> {
        let required_params = self
            .parameters()
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();

        if let Some(params) = input.parameters.as_object() {
            for param in required_params {
                if !params.contains_key(&param) {
                    return Err(crate::error::KowalskiError::ToolInvalidInput(format!(
                        "Missing required parameter: {}",
                        param
                    )));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub parameters: serde_json::Value,
    pub reasoning: Option<String>,
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
        Self {
            task_type,
            content,
            parameters,
        }
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
