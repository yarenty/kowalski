//! MCP Streamable HTTP (JSON + SSE) server: DataFusion tools over a registered CSV.

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::routing::post;
use axum::{Router, response::IntoResponse};
use datafusion::arrow::datatypes::Schema;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

pub const MCP_SESSION_HEADER: &str = "mcp-session-id";
pub const ACCEPT_STREAMABLE: &str = "application/json, text/event-stream";

#[derive(Clone)]
pub struct AppState {
    pub ctx: Arc<SessionContext>,
    pub table: String,
    pub session_id: String,
}

impl AppState {
    pub fn new(ctx: Arc<SessionContext>, table: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            ctx,
            table: table.into(),
            session_id: session_id.into(),
        }
    }
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/", post(mcp_post))
        .with_state(state)
}

/// Streamable HTTP: respond with SSE when the client advertises `text/event-stream`.
pub fn wants_sse(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

pub fn response_for_envelope(
    headers: &HeaderMap,
    session_id: &str,
    envelope: Value,
) -> Response<Body> {
    let body_str = match serde_json::to_string(&envelope) {
        Ok(s) => s,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))
                .unwrap();
        }
    };

    let mut builder = Response::builder().status(StatusCode::OK);
    if let Ok(v) = HeaderValue::from_str(session_id) {
        builder = builder.header(MCP_SESSION_HEADER, v);
    }

    if wants_sse(headers) {
        let sse = format!("data: {}\n\n", body_str);
        builder
            .header(CONTENT_TYPE, "text/event-stream")
            .body(Body::from(sse))
            .unwrap()
    } else {
        builder
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body_str))
            .unwrap()
    }
}

async fn mcp_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let body_str = String::from_utf8_lossy(&body);
    let v: Value = match serde_json::from_str(body_str.trim()) {
        Ok(x) => x,
        Err(e) => {
            return response_for_envelope(
                &headers,
                &state.session_id,
                json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("parse error: {}", e) }
                }),
            );
        }
    };

    let method = v["method"].as_str().unwrap_or("");
    if v.get("id").is_none() && method == "notifications/initialized" {
        let mut res = Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(Body::empty())
            .unwrap();
        if let Ok(h) = HeaderValue::from_str(&state.session_id) {
            res.headers_mut().insert(MCP_SESSION_HEADER, h);
        }
        return res;
    }

    let envelope = dispatch_mcp(&state, v).await;
    response_for_envelope(&headers, &state.session_id, envelope)
}

async fn dispatch_mcp(state: &AppState, body: Value) -> Value {
    let id = body.get("id").cloned().unwrap_or(json!(1));
    let method = body["method"].as_str().unwrap_or("");

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-03-26",
            "serverInfo": {
                "name": "kowalski-mcp-datafusion",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": { "tools": {} }
        }),
        "tools/list" => tools_list_json(),
        "tools/call" => match run_tool_call(state, &body).await {
            Ok(v) => v,
            Err(e) => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32000, "message": e }
                });
            }
        },
        _ => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "method not found" }
            });
        }
    };

    json!({ "jsonrpc": "2.0", "id": id, "result": result })
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
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

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
