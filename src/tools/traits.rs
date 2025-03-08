use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInput {
    pub query: String,
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolOutput {
    pub content: String,
    pub metadata: HashMap<String, Value>,
    pub source: Option<String>,
}