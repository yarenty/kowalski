use crate::config::{McpServerConfig, McpTransport};
use crate::error::KowalskiError;
use crate::mcp::client::McpClient;
use crate::mcp::stdio::McpStdioClient;
use crate::mcp::tool::McpToolProxy;
use crate::mcp::types::{CallToolResponse, McpToolDescription};
use crate::tools::Tool;
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;

/// Active transport for an MCP tool binding.
#[derive(Clone)]
pub enum McpConnection {
    Http(McpClient),
    Stdio(McpStdioClient),
}

impl McpConnection {
    async fn list_tools(&self) -> Result<Vec<McpToolDescription>, KowalskiError> {
        match self {
            Self::Http(c) => c.list_tools().await,
            Self::Stdio(c) => c.list_tools().await,
        }
    }

    async fn call_tool(
        &self,
        remote_name: &str,
        args: &serde_json::Value,
    ) -> Result<CallToolResponse, KowalskiError> {
        match self {
            Self::Http(c) => c.call_tool(remote_name, args).await,
            Self::Stdio(c) => c.call_tool(remote_name, args).await,
        }
    }
}

/// A binding between a public tool name and the MCP server/client that owns it.
#[derive(Clone)]
pub struct McpToolBinding {
    pub display_name: String,
    pub remote_name: String,
    pub server_name: String,
    pub description: McpToolDescription,
    pub client: McpConnection,
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
            let conn = if matches!(server.transport, McpTransport::Stdio) {
                match McpStdioClient::connect(server).await {
                    Ok(c) => McpConnection::Stdio(c),
                    Err(err) => {
                        warn!("Failed to connect stdio MCP '{}': {}", server.name, err);
                        continue;
                    }
                }
            } else {
                match McpClient::connect_server(server).await {
                    Ok(c) => McpConnection::Http(c),
                    Err(err) => {
                        warn!("Failed to connect to MCP server '{}': {}", server.name, err);
                        continue;
                    }
                }
            };

            match conn.list_tools().await {
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
                            client: conn.clone(),
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
        let binding = self.tools.get(tool_name).ok_or_else(|| {
            KowalskiError::ToolExecution(format!("Unknown MCP tool {}", tool_name))
        })?;

        let response = binding.client.call_tool(&binding.remote_name, args).await?;
        Ok(response.normalized_content())
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
