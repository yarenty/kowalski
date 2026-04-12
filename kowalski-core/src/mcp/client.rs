use crate::error::KowalskiError;
use crate::mcp::types::{CallToolResponse, InitializeResult, ToolListResult};
use log::{debug, warn};
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Thin JSON-RPC client for MCP servers (HTTP/SSE transport).
#[derive(Debug, Clone)]
pub struct McpClient {
    name: String,
    base_url: Url,
    http: reqwest::Client,
    id_counter: Arc<AtomicU64>,
    init: Arc<InitializeResult>,
}

impl McpClient {
    pub async fn connect(name: impl Into<String>, url: &str) -> Result<Self, KowalskiError> {
        let base_url = Url::parse(url)?;
        let http = reqwest::Client::builder()
            .build()
            .map_err(KowalskiError::Request)?;

        let mut client = Self {
            name: name.into(),
            base_url,
            http,
            id_counter: Arc::new(AtomicU64::new(1)),
            init: Arc::new(InitializeResult::default()),
        };

        match client.initialize().await {
            Ok(info) => client.init = Arc::new(info),
            Err(err) => warn!("Failed to initialize MCP server '{}': {}", client.name, err),
        }

        Ok(client)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    async fn initialize(&self) -> Result<InitializeResult, KowalskiError> {
        let params = json!({
            "clientInfo": {
                "name": "Kowalski",
                "version": env!("CARGO_PKG_VERSION")
            },
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": true,
            }
        });

        self.send_request::<InitializeResult>("initialize", Some(params))
            .await
    }

    pub async fn list_tools(&self) -> Result<Vec<crate::mcp::types::McpToolDescription>, KowalskiError> {
        let result: ToolListResult = self.send_request("tools/list", None).await?;
        Ok(result.tools)
    }

    pub async fn call_tool(
        &self,
        tool: &str,
        arguments: &serde_json::Value,
    ) -> Result<CallToolResponse, KowalskiError> {
        let params = json!({
            "name": tool,
            "arguments": arguments,
        });
        self.send_request("tools/call", Some(params)).await
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<T, KowalskiError> {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or_else(|| json!({})),
        });

        debug!(
            "MCP {} -> {} payload: {}",
            self.name,
            method,
            payload
        );

        let response = self
            .http
            .post(self.base_url.clone())
            .json(&payload)
            .send()
            .await
            .map_err(KowalskiError::Request)?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(KowalskiError::Request)?;

        if !status.is_success() {
            return Err(KowalskiError::Network(format!(
                "MCP server {} returned HTTP {}: {}",
                self.name, status, body
            )));
        }

        if let Some(error) = body.get("error") {
            return Err(KowalskiError::ToolExecution(format!(
                "MCP error {}: {}",
                self.name, error
            )));
        }

        let result = body
            .get("result")
            .cloned()
            .ok_or_else(|| KowalskiError::ToolExecution("Missing result field".into()))?;

        serde_json::from_value(result).map_err(KowalskiError::Json)
    }
}
