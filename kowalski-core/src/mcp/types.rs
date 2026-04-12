use serde::{Deserialize, Serialize};

/// Basic information returned by an MCP server during initialization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitializeResult {
    #[serde(default, rename = "serverInfo")]
    pub server: Option<ServerInfo>,
    #[serde(default, rename = "protocolVersion")]
    pub protocol_version: Option<String>,
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
    #[serde(default)]
    pub description: String,
    /// MCP spec uses camelCase `inputSchema`; some servers emit snake_case.
    #[serde(default, rename = "inputSchema", alias = "input_schema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolListResult {
    #[serde(default)]
    pub tools: Vec<McpToolDescription>,
}

/// Raw response of the `tools/call` method. The MCP spec wraps tool output in a
/// `content` array where each entry can be text/json/etc.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CallToolResponse {
    #[serde(default)]
    pub content: serde_json::Value,
}

impl CallToolResponse {
    /// Normalize MCP `content` (often an array of `{type,text}` blocks) into a single JSON value for the agent loop.
    pub fn normalized_content(self) -> serde_json::Value {
        match self.content {
            serde_json::Value::Array(items) if !items.is_empty() => {
                let mut parts = Vec::new();
                for item in items {
                    if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                        parts.push(t.to_string());
                    } else {
                        parts.push(item.to_string());
                    }
                }
                serde_json::Value::String(parts.join("\n"))
            }
            other => other,
        }
    }
}
