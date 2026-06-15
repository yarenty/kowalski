//! **Stateless** Streamable HTTP (JSON + SSE) MCP server: DataFusion tools over a registered CSV.
//!
//! Transport (stdio / stateless HTTP, SSE framing, parse errors) lives in
//! [`kowalski_mcp_transport`]; this crate only implements the DataFusion tool dispatch via
//! [`McpHandler`]. No `Mcp-Session-Id` is issued or required — every POST is independent.

use axum::Router;
use datafusion::arrow::datatypes::Schema;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::*;
use kowalski_mcp_transport::{McpHandler, http_router};
use serde_json::{Value, json};
use std::sync::Arc;

/// `Accept` value clients should send (re-exported from the shared transport).
pub use kowalski_mcp_transport::ACCEPT_STREAMABLE;

/// MCP protocol version reported on `initialize`.
pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// DataFusion-backed MCP handler: a registered table plus the session context to query it.
#[derive(Clone)]
pub struct AppState {
    pub ctx: Arc<SessionContext>,
    pub table: String,
}

impl AppState {
    pub fn new(ctx: Arc<SessionContext>, table: impl Into<String>) -> Self {
        Self {
            ctx,
            table: table.into(),
        }
    }
}

impl McpHandler for AppState {
    fn handle(&self, request: Value) -> impl std::future::Future<Output = Option<Value>> + Send {
        let state = self.clone();
        async move { dispatch_mcp(&state, request).await }
    }
}

/// Build the stateless Streamable HTTP router for this DataFusion table.
pub fn app_router(state: AppState) -> Router {
    http_router(Arc::new(state))
}

async fn dispatch_mcp(state: &AppState, body: Value) -> Option<Value> {
    // JSON-RPC notifications (no `id`, e.g. notifications/initialized) get no reply.
    let id = body.get("id")?.clone();
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {
                "name": "kowalski-mcp-datafusion",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": { "tools": {} }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list_json()),
        "tools/call" => run_tool_call(state, &body).await,
        other => {
            return Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {other}") }
            }));
        }
    };

    Some(match result {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32000, "message": e }
        }),
    })
}

fn tools_list_json() -> Value {
    json!({
        "tools": [
            {
                "name": "query_sql",
                "description": "Run an arbitrary SQL query against the registered table and return a pretty-printed result grid.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "sql": { "type": "string", "description": "SQL statement (registered table is available under the configured name)" }
                    },
                    "required": ["sql"]
                }
            },
            {
                "name": "get_schema",
                "description": "Return column names, Arrow data types, and nullability for the registered table (LIMIT 0 scan).",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
            {
                "name": "column_statistics",
                "description": "High-level statistics for every column: count, null_count, mean, std, min, max (DataFusion describe).",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            }
        ]
    })
}

fn schema_to_json(table: &str, schema: &Schema) -> Value {
    let columns: Vec<Value> = schema
        .fields()
        .iter()
        .map(|f| {
            json!({
                "name": f.name(),
                "data_type": format!("{}", f.data_type()),
                "nullable": f.is_nullable(),
            })
        })
        .collect();
    json!({
        "table": table,
        "columns": columns
    })
}

async fn run_tool_call(state: &AppState, body: &Value) -> Result<Value, String> {
    let params = body.get("params").cloned().unwrap_or_else(|| json!({}));
    let name = params["name"].as_str().unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "query_sql" => {
            let sql = args["sql"]
                .as_str()
                .ok_or_else(|| "missing arguments.sql".to_string())?;
            let df = state.ctx.sql(sql).await.map_err(|e| e.to_string())?;
            let batches = df.collect().await.map_err(|e| e.to_string())?;
            let text = pretty_format_batches(&batches)
                .map_err(|e| e.to_string())?
                .to_string();
            Ok(json!({
                "content": [{ "type": "text", "text": text }]
            }))
        }
        "get_schema" => {
            let sql = format!("SELECT * FROM {} LIMIT 0", state.table);
            let df = state.ctx.sql(&sql).await.map_err(|e| e.to_string())?;
            let j = schema_to_json(&state.table, df.schema().as_arrow());
            Ok(json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&j).unwrap_or_else(|_| j.to_string()) }]
            }))
        }
        "column_statistics" => {
            let sql = format!("SELECT * FROM {}", state.table);
            let df = state.ctx.sql(&sql).await.map_err(|e| e.to_string())?;
            let desc = df.describe().await.map_err(|e| e.to_string())?;
            let batches = desc.collect().await.map_err(|e| e.to_string())?;
            let text = pretty_format_batches(&batches)
                .map_err(|e| e.to_string())?
                .to_string();
            Ok(json!({
                "content": [{ "type": "text", "text": text }]
            }))
        }
        _ => Err(format!("unknown tool: {}", name)),
    }
}
