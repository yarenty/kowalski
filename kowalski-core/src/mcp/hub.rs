use crate::config::McpServerConfig;
use crate::error::KowalskiError;
use crate::mcp::client::McpClient;
use crate::mcp::tool::McpToolProxy;
use crate::mcp::types::McpToolDescription;
use crate::tools::Tool;
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;

/// A binding between a public tool name and the MCP server/client that owns it.
#[derive(Debug, Clone)]
pub struct McpToolBinding {
    pub display_name: String,
    pub remote_name: String,
    pub server_name: String,
    pub description: McpToolDescription,
    pub client: McpClient,
}

#[derive(Clone)]
pub struct McpHub {
    tools: HashMap<String, McpToolBinding>,
}

impl McpHub {
    pub async fn new(servers: &[McpServerConfig]) -> Result<Option<Arc<Self>>, KowalskiError> {
        if servers.is_empty() {
            return Ok(None);
        }

        let mut bindings = HashMap::new();

        for server in servers {
            let client = match McpClient::connect(&server.name, &server.url).await {
                Ok(c) => c,
                Err(err) => {
                    warn!(
                        "Failed to connect to MCP server '{}': {}",
                        server.name, err
                    );
                    continue;
                }
            };

            match client.list_tools().await {
                Ok(tools) => {
                    info!(
                        "MCP server '{}' exposed {} tool(s)",
                        server.name,
                        tools.len()
                    );
                    for tool in tools {
                        let display_name =
                            McpHub::resolve_tool_name(&tool.name, &server.name, &bindings);
                        let binding = McpToolBinding {
                            remote_name: tool.name.clone(),
                            display_name: display_name.clone(),
                            server_name: server.name.clone(),
                            description: tool.clone(),
                            client: client.clone(),
                        };
                        bindings.insert(display_name, binding);
                    }
                }
                Err(err) => {
                    warn!(
                        "Failed to list tools from MCP server '{}': {}",
                        server.name, err
                    );
                }
            }
        }

        if bindings.is_empty() {
            return Ok(None);
        }

        Ok(Some(Arc::new(Self { tools: bindings })))
    }

    fn resolve_tool_name(
        base: &str,
        server_name: &str,
        current: &HashMap<String, McpToolBinding>,
    ) -> String {
        if !current.contains_key(base) {
            return base.to_string();
        }
        format!("{}::{}", server_name, base)
    }

    pub fn iter_bindings(&self) -> impl Iterator<Item = &McpToolBinding> {
        self.tools.values()
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, KowalskiError> {
        let binding = self
            .tools
            .get(tool_name)
            .ok_or_else(|| KowalskiError::ToolExecution(format!("Unknown MCP tool {}", tool_name)))?;

        let response = binding
            .client
            .call_tool(&binding.remote_name, args)
            .await?;
        Ok(response.content)
    }

    pub fn into_tool_proxies(self: &Arc<Self>) -> Vec<Box<dyn Tool + Send + Sync>> {
        self.iter_bindings()
            .map(|binding| {
                Box::new(McpToolProxy::new(
                    self.clone(),
                    binding.display_name.clone(),
                    binding.description.clone(),
                )) as Box<dyn Tool + Send + Sync>
            })
            .collect()
    }
}
