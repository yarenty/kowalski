//! MCP over **stdio**: newline-delimited JSON-RPC 2.0 (one request / one response per line).

use crate::config::{McpServerConfig, McpTransport};
use crate::error::KowalskiError;
use crate::mcp::types::{CallToolResponse, InitializeResult, ToolListResult};
use log::debug;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// JSON-RPC line client for a local MCP subprocess.
#[derive(Clone)]
pub struct McpStdioClient {
    inner: Arc<StdioInner>,
}

struct StdioInner {
    name: String,
    #[allow(dead_code)]
    child: tokio::process::Child,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
    id: AtomicU64,
}

impl McpStdioClient {
    pub async fn connect(server: &McpServerConfig) -> Result<Self, KowalskiError> {
        if !matches!(server.transport, McpTransport::Stdio) {
            return Err(KowalskiError::Configuration(
                "McpStdioClient::connect: transport must be stdio".into(),
            ));
        }
        if server.command.is_empty() {
            return Err(KowalskiError::Configuration(
                "stdio MCP requires `command` = [program, ...args]".into(),
            ));
        }
        let mut cmd = tokio::process::Command::new(&server.command[0]);
        cmd.args(&server.command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|e| {
            KowalskiError::Configuration(format!("stdio MCP spawn {}: {e}", server.command[0]))
        })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| KowalskiError::Configuration("stdio MCP: stdin not available".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| {
            KowalskiError::Configuration("stdio MCP: stdout not available".into())
        })?;

        let client = Self {
            inner: Arc::new(StdioInner {
                name: server.name.clone(),
                child,
                stdin: Mutex::new(stdin),
                stdout: Mutex::new(BufReader::new(stdout)),
                id: AtomicU64::new(1),
            }),
        };

        let _info: InitializeResult = client
            .send_request(
                "initialize",
                Some(json!({
                    "clientInfo": { "name": "Kowalski", "version": env!("CARGO_PKG_VERSION") },
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": { "tools": true },
                })),
            )
            .await?;
        client
            .send_notification("notifications/initialized", Some(json!({})))
            .await?;

        Ok(client)
    }

    async fn send_notification(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), KowalskiError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or_else(|| json!({})),
        });
        self.write_line(&payload).await
    }

    async fn write_line(&self, v: &Value) -> Result<(), KowalskiError> {
        let mut line = serde_json::to_string(v).map_err(KowalskiError::Json)?;
        line.push('\n');
        let mut g = self.inner.stdin.lock().await;
        g.write_all(line.as_bytes())
            .await
            .map_err(|e| KowalskiError::Network(format!("stdio MCP write: {e}")))?;
        g.flush()
            .await
            .map_err(|e| KowalskiError::Network(format!("stdio MCP flush: {e}")))?;
        Ok(())
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<T, KowalskiError> {
        let id = self.inner.id.fetch_add(1, Ordering::SeqCst);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or_else(|| json!({})),
        });
        debug!("MCP stdio {} -> {}", self.inner.name, method);
        self.write_line(&payload).await?;

        let mut line = String::new();
        self.inner
            .stdout
            .lock()
            .await
            .read_line(&mut line)
            .await
            .map_err(|e| KowalskiError::Network(format!("stdio MCP read: {e}")))?;
        let body: Value = serde_json::from_str(line.trim()).map_err(KowalskiError::Json)?;
        if let Some(err) = body.get("error") {
            return Err(KowalskiError::ToolExecution(format!(
                "MCP stdio {}: {}",
                self.inner.name, err
            )));
        }
        let result = body.get("result").cloned().ok_or_else(|| {
            KowalskiError::ToolExecution("Missing result in stdio MCP reply".into())
        })?;
        serde_json::from_value(result).map_err(KowalskiError::Json)
    }

    pub async fn list_tools(
        &self,
    ) -> Result<Vec<crate::mcp::types::McpToolDescription>, KowalskiError> {
        let result: ToolListResult = self.send_request("tools/list", None).await?;
        Ok(result.tools)
    }

    pub async fn call_tool(
        &self,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> Result<CallToolResponse, KowalskiError> {
        let params = json!({ "name": tool, "arguments": arguments });
        self.send_request("tools/call", Some(params)).await
    }
}
