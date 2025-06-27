use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool execution failed: {0}")]
    Execution(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub task_type: String,
    pub content: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub result: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

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

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Vec<ToolParameter>;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
    fn validate_input(&self, input: &ToolInput) -> Result<(), ToolError> {
        let required_params = self
            .parameters()
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();

        if let Some(params) = input.parameters.as_object() {
            for param in required_params {
                if !params.contains_key(&param) {
                    return Err(ToolError::InvalidInput(format!(
                        "Missing required parameter: {}",
                        param
                    )));
                }
            }
        }

        Ok(())
    }
}

pub struct ToolManager {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolManager {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register_tool<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.push(Box::new(tool));
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub async fn execute_tool(
        &self,
        name: &str,
        input: ToolInput,
    ) -> Result<ToolOutput, ToolError> {
        if let Some(tool) = self.get_tool(name) {
            tool.validate_input(&input)?;
            tool.execute(input).await
        } else {
            Err(ToolError::NotFound(name.to_string()))
        }
    }

    pub fn list_tools(&self) -> Vec<(String, String)> {
        self.tools
            .iter()
            .map(|t| (t.name().to_string(), t.description().to_string()))
            .collect()
    }
}
