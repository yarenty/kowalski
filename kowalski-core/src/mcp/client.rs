use crate::config::{McpServerConfig, McpTransport};
use crate::error::KowalskiError;
use crate::mcp::types::{CallToolResponse, InitializeResult, ToolListResult};
use log::{debug, warn};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use reqwest::{RequestBuilder, Response, Url};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const ACCEPT_STREAMABLE: &str = "application/json, text/event-stream";
const HEADER_MCP_SESSION_ID: &str = "mcp-session-id";

/// Thin JSON-RPC client for MCP servers ([Streamable HTTP](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports)
/// and legacy POST-only endpoints).
#[derive(Debug, Clone)]
pub struct McpClient {
    name: String,
    base_url: Url,
    http: reqwest::Client,
    id_counter: Arc<AtomicU64>,
    init: Arc<InitializeResult>,
    /// From `Mcp-Session-Id` on `initialize` and subsequent streamable HTTP responses.
    session_id: Arc<Mutex<Option<String>>>,
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
    /// and transport hint. Uses **Streamable HTTP** (`Accept: application/json, text/event-stream`),
    /// captures [`Mcp-Session-Id`](https://modelcontextprotocol.io/specification/2025-03-26/basic/transports#session-management),
    /// and parses both JSON and SSE bodies. [`McpTransport::Sse`] is treated as streamable-capable
    /// (same POST path; no separate legacy GET bootstrap in this build).
    pub async fn connect_server(server: &McpServerConfig) -> Result<Self, KowalskiError> {
        let base_url = Url::parse(&server.url)?;
        let http = build_http_client(&server.headers)?;

        if matches!(server.transport, McpTransport::Sse) {
            debug!(
                "MCP server '{}': transport=sse — using Streamable HTTP POST + optional SSE response body",
                server.name
            );
        }

        let mut client = Self {
            name: server.name.clone(),
            base_url,
            http,
            id_counter: Arc::new(AtomicU64::new(1)),
            init: Arc::new(InitializeResult::default()),
            session_id: Arc::new(Mutex::new(None)),
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

    fn apply_streamable_headers(&self, mut req: RequestBuilder) -> RequestBuilder {
        req = req.header(ACCEPT, ACCEPT_STREAMABLE);
        if let Ok(guard) = self.session_id.lock() {
            if let Some(ref sid) = *guard {
                if let Ok(v) = HeaderValue::from_str(sid) {
                    req = req.header(HEADER_MCP_SESSION_ID, v);
                }
            }
        }
        req
    }

    fn capture_session_from_response(&self, response: &Response) {
        if let Some(h) = response.headers().get(HEADER_MCP_SESSION_ID) {
            if let Ok(s) = h.to_str() {
                if let Ok(mut guard) = self.session_id.lock() {
                    *guard = Some(s.to_string());
                    debug!("MCP '{}' session id updated from response", self.name);
                }
            }
        }
    }

    async fn initialize(&self) -> Result<InitializeResult, KowalskiError> {
        let params = json!({
            "clientInfo": {
                "name": "Kowalski",
                "version": env!("CARGO_PKG_VERSION")
            },
            "protocolVersion": MCP_PROTOCOL_VERSION,
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
    /// Streamable HTTP may respond with **202 Accepted** and an empty body.
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
            .apply_streamable_headers(
                self.http
                    .post(self.base_url.clone())
                    .json(&payload),
            )
            .send()
            .await
            .map_err(KowalskiError::Request)?;

        self.capture_session_from_response(&response);

        let status = response.status();
        if status == reqwest::StatusCode::ACCEPTED {
            return Ok(());
        }
        if status.is_success() {
            return Ok(());
        }

        let body = response.text().await.unwrap_or_default();
        Err(KowalskiError::Network(format!(
            "MCP notification {} returned HTTP {}: {}",
            method, status, body
        )))
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
            .apply_streamable_headers(
                self.http
                    .post(self.base_url.clone())
                    .json(&payload),
            )
            .send()
            .await
            .map_err(KowalskiError::Request)?;

        self.capture_session_from_response(&response);

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            if let Ok(mut g) = self.session_id.lock() {
                g.take();
            }
            return Err(KowalskiError::Network(format!(
                "MCP server {} returned HTTP 404 (session may have expired)",
                self.name
            )));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Network(format!(
                "MCP server {} returned HTTP {}: {}",
                self.name, status, body
            )));
        }

        let body_value = self.parse_streamable_body(response, id).await?;

        if let Some(error) = body_value.get("error") {
            return Err(KowalskiError::ToolExecution(format!(
                "MCP error {}: {}",
                self.name, error
            )));
        }

        let result = body_value
            .get("result")
            .cloned()
            .ok_or_else(|| KowalskiError::ToolExecution("Missing result field".into()))?;

        serde_json::from_value(result).map_err(KowalskiError::Json)
    }

    /// Parse Streamable HTTP response: `application/json` or `text/event-stream` (SSE) containing JSON-RPC.
    async fn parse_streamable_body(
        &self,
        response: Response,
        expected_id: u64,
    ) -> Result<serde_json::Value, KowalskiError> {
        let ctype = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();

        if ctype.contains("text/event-stream") {
            let text = response.text().await.map_err(KowalskiError::Request)?;
            return parse_sse_jsonrpc_response(&text, expected_id);
        }

        let body: serde_json::Value = response.json().await.map_err(KowalskiError::Request)?;
        Ok(body)
    }
}

fn jsonrpc_id_matches(msg: &serde_json::Value, expected_id: u64) -> bool {
    match msg.get("id") {
        Some(serde_json::Value::Number(n)) => n.as_u64() == Some(expected_id),
        Some(serde_json::Value::String(s)) => s.parse::<u64>().ok() == Some(expected_id),
        _ => false,
    }
}

/// Extract the JSON-RPC object for `expected_id` from an SSE body (`data: ...` lines).
fn parse_sse_jsonrpc_response(sse_body: &str, expected_id: u64) -> Result<serde_json::Value, KowalskiError> {
    for line in sse_body.lines() {
        let line = line.trim();
        let rest = line
            .strip_prefix("data:")
            .map(str::trim)
            .or_else(|| line.strip_prefix("data: ").map(str::trim));
        let Some(candidate) = rest else {
            continue;
        };
        if candidate.is_empty() || candidate == "[DONE]" {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
            if jsonrpc_id_matches(&v, expected_id) {
                return Ok(v);
            }
        }
    }
    Err(KowalskiError::ToolExecution(format!(
        "SSE response contained no JSON-RPC message with id {expected_id}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_parses_jsonrpc_with_matching_id() {
        let sse = r#"data: {"jsonrpc":"2.0","id":7,"result":{"tools":[]}}

"#;
        let v = parse_sse_jsonrpc_response(sse, 7).unwrap();
        assert!(v.get("result").is_some());
    }
}
