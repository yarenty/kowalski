use crate::config::{McpServerConfig, McpTransport};
use crate::error::KowalskiError;
use crate::mcp::types::{CallToolResponse, InitializeResult, ToolListResult};
use log::{debug, warn};
use reqwest::Url;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
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

fn build_http_client(headers: &HashMap<String, String>) -> Result<reqwest::Client, KowalskiError> {
    let mut map = HeaderMap::new();
    for (key, value) in headers {
        let hname = HeaderName::from_str(key.trim()).map_err(|e| {
            KowalskiError::Configuration(format!("Invalid MCP header name '{key}': {e}"))
        })?;
        let hval = HeaderValue::from_str(value).map_err(|e| {
            KowalskiError::Configuration(format!("Invalid MCP header value for '{key}': {e}"))
        })?;
        map.insert(hname, hval);
    }
    reqwest::Client::builder()
        .default_headers(map)
        .build()
        .map_err(KowalskiError::Request)
}

impl McpClient {
    /// Connect using full server config: URL, optional [`McpServerConfig::headers`] (e.g. auth),
    /// and transport hint (full SSE session is not implemented yet; JSON-RPC uses HTTP POST).
    pub async fn connect_server(server: &McpServerConfig) -> Result<Self, KowalskiError> {
        let base_url = Url::parse(&server.url)?;
        let http = build_http_client(&server.headers)?;

        match server.transport {
            McpTransport::Sse => {
                debug!(
                    "MCP server '{}': transport=sse; JSON-RPC still uses HTTP POST on {} until full SSE transport is implemented",
                    server.name, server.url
                );
            }
            McpTransport::Http => {}
        }

        let mut client = Self {
            name: server.name.clone(),
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

    /// Convenience: no extra headers, HTTP-style POST transport.
    pub async fn connect(name: impl Into<String>, url: &str) -> Result<Self, KowalskiError> {
        Self::connect_server(&McpServerConfig {
            name: name.into(),
            url: url.to_string(),
            transport: McpTransport::Http,
            headers: HashMap::new(),
        })
        .await
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

        let info = self
            .send_request::<InitializeResult>("initialize", Some(params))
            .await?;

        if let Err(e) = self
            .send_notification("notifications/initialized", Some(json!({})))
            .await
        {
            warn!(
                "MCP '{}' lifecycle: notifications/initialized failed: {}",
                self.name, e
            );
        }

        Ok(info)
    }

    /// JSON-RPC notification (no `id`). Used after successful `initialize` per MCP lifecycle.
    async fn send_notification(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), KowalskiError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or_else(|| json!({})),
        });

        debug!(
            "MCP {} notification -> {} payload: {}",
            self.name, method, payload
        );

        let response = self
            .http
            .post(self.base_url.clone())
            .json(&payload)
            .send()
            .await
            .map_err(KowalskiError::Request)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Network(format!(
                "MCP notification {} returned HTTP {}: {}",
                method, status, body
            )));
        }
        Ok(())
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

        debug!("MCP {} -> {} payload: {}", self.name, method, payload);

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
