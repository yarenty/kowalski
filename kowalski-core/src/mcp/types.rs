use serde::{Deserialize, Serialize};

/// Basic information returned by an MCP server during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitializeResult {
    #[serde(default)]
    pub server: Option<ServerInfo>,
    #[serde(default)]
    pub capabilities: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerInfo {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// Tool description returned by `tools/list`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescription {
    pub name: String,
    pub description: String,
    #[serde(rename = "input_schema", default)]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolListResult {
    #[serde(default)]
    pub tools: Vec<McpToolDescription>,
}

/// Raw response of the `tools/call` method. The MCP spec wraps tool output in a
/// `content` array where each entry can be text/json/etc. For now we simply keep
/// the raw JSON payload and hand it back to the caller.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallToolResponse {
    #[serde(default)]
    pub content: serde_json::Value,
}
