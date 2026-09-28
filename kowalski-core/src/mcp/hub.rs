use crate::config::{McpServerConfig, McpTransport};
use crate::error::KowalskiError;
use crate::mcp::client::McpClient;
use crate::mcp::stdio::McpStdioClient;
use crate::mcp::tool::McpToolProxy;
use crate::mcp::types::{CallToolResponse, McpToolDescription};
use crate::tools::Tool;
use crate::tools::manager::{ToolManager, ToolRefresher};
use log::{info, warn};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

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

/// Connects to the configured MCP servers and serves their tools under one namespace.
///
/// A server that cannot be reached when the hub starts (tableski launched after kowalski, a
/// gateway still booting) is kept as *pending*: [`McpHub::attach`] retries it in the background
/// with backoff until it answers, and a lookup of a tool that is not there yet retries at once
/// (rate-limited), so its tools appear without restarting kowalski.
pub struct McpHub {
    tools: RwLock<HashMap<String, McpToolBinding>>,
    /// Servers not connected yet; the async lock also keeps two retries from racing.
    pending: tokio::sync::Mutex<Vec<McpServerConfig>>,
    last_retry: std::sync::Mutex<Option<Instant>>,
}

/// Shortest gap between two on-demand retries of the pending servers.
const RETRY_MIN_GAP: Duration = Duration::from_secs(1);
/// Background retry delays: 5 s, doubling, at most 60 s.
const RETRY_FIRST: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(60);

impl McpHub {
    /// `None` only when no server is configured; unreachable servers stay pending.
    pub async fn new(servers: &[McpServerConfig]) -> Result<Option<Arc<Self>>, KowalskiError> {
        if servers.is_empty() {
            return Ok(None);
        }
        let hub = Arc::new(Self {
            tools: RwLock::new(HashMap::new()),
            pending: tokio::sync::Mutex::new(Vec::new()),
            last_retry: std::sync::Mutex::new(None),
        });
        let mut pending = Vec::new();
        for server in servers {
            if !hub.connect(server).await {
                pending.push(server.clone());
            }
        }
        *hub.pending.lock().await = pending;
        Ok(Some(hub))
    }

    /// Connect one server and add its tools; `false` when it could not be reached.
    async fn connect(&self, server: &McpServerConfig) -> bool {
        let conn = if matches!(server.transport, McpTransport::Stdio) {
            match McpStdioClient::connect(server).await {
                Ok(c) => McpConnection::Stdio(c),
                Err(err) => {
                    warn!("Failed to connect stdio MCP '{}': {}", server.name, err);
                    return false;
                }
            }
        } else {
            match McpClient::connect_server(server).await {
                Ok(c) => McpConnection::Http(c),
                Err(err) => {
                    warn!("Failed to connect to MCP server '{}': {}", server.name, err);
                    return false;
                }
            }
        };
        let tools = match conn.list_tools().await {
            Ok(tools) => tools,
            Err(err) => {
                warn!("Failed to list tools from MCP server '{}': {}", server.name, err);
                return false;
            }
        };
        info!("MCP server '{}' exposed {} tool(s)", server.name, tools.len());
        let mut bindings = self.tools.write().unwrap_or_else(|e| e.into_inner());
        for tool in tools {
            let display_name = McpHub::resolve_tool_name(&tool.name, &server.name, &bindings);
            let binding = McpToolBinding {
                remote_name: tool.name.clone(),
                display_name: display_name.clone(),
                server_name: server.name.clone(),
                description: tool.clone(),
                client: conn.clone(),
            };
            bindings.insert(display_name, binding);
        }
        true
    }

    /// Try the pending servers again; returns how many connected.
    pub async fn retry_pending(&self) -> usize {
        let mut pending = self.pending.lock().await;
        if pending.is_empty() {
            return 0;
        }
        *self.last_retry.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        let mut still = Vec::new();
        let mut connected = 0;
        for server in pending.drain(..) {
            if self.connect(&server).await {
                info!("MCP server '{}' connected after a retry", server.name);
                connected += 1;
            } else {
                still.push(server);
            }
        }
        *pending = still;
        connected
    }

    /// Names of the servers that are not connected yet.
    pub async fn pending_servers(&self) -> Vec<String> {
        self.pending.lock().await.iter().map(|s| s.name.clone()).collect()
    }

    fn retried_recently(&self) -> bool {
        self.last_retry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some_and(|t| t.elapsed() < RETRY_MIN_GAP)
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

    /// A snapshot of the current tool bindings.
    pub fn bindings(&self) -> Vec<McpToolBinding> {
        self.tools.read().unwrap_or_else(|e| e.into_inner()).values().cloned().collect()
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, KowalskiError> {
        let binding = self
            .tools
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(tool_name)
            .cloned()
            .ok_or_else(|| KowalskiError::ToolExecution(format!("Unknown MCP tool {}", tool_name)))?;
        let response = binding.client.call_tool(&binding.remote_name, args).await?;
        Ok(response.normalized_content())
    }

    pub fn into_tool_proxies(self: &Arc<Self>) -> Vec<Box<dyn Tool + Send + Sync>> {
        self.bindings()
            .into_iter()
            .map(|binding| {
                Box::new(McpToolProxy::new(
                    self.clone(),
                    binding.display_name.clone(),
                    binding.description.clone(),
                )) as Box<dyn Tool + Send + Sync>
            })
            .collect()
    }

    /// Register the hub's tools with `tools`, and keep them coming: servers that were not
    /// reachable are retried in the background (5 s, doubling to 60 s, until all answer) and
    /// whenever `tools` is asked for a tool it does not have.
    pub fn attach(self: &Arc<Self>, tools: &ToolManager) {
        let refresher = Arc::new(HubRefresher { hub: self.clone(), tools: tools.clone() });
        refresher.register_new();
        tools.set_refresher(refresher.clone());
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(async move {
                let mut delay = RETRY_FIRST;
                while !refresher.hub.pending.lock().await.is_empty() {
                    tokio::time::sleep(delay).await;
                    refresher.retry().await;
                    delay = (delay * 2).min(RETRY_MAX);
                }
            });
        }
    }
}

/// Registers newly connected MCP tools with a [`ToolManager`].
struct HubRefresher {
    hub: Arc<McpHub>,
    tools: ToolManager,
}

impl HubRefresher {
    fn register_new(&self) {
        for proxy in self.hub.into_tool_proxies() {
            if self.tools.get(proxy.name()).is_none() {
                self.tools.register_boxed(proxy);
            }
        }
    }

    async fn retry(&self) -> usize {
        let connected = self.hub.retry_pending().await;
        if connected > 0 {
            self.register_new();
        }
        connected
    }
}

#[async_trait::async_trait]
impl ToolRefresher for HubRefresher {
    async fn refresh(&self) -> bool {
        if self.hub.retried_recently() {
            return false;
        }
        self.retry().await > 0
    }
}
