//! MCP **Streamable HTTP** server (JSON or **SSE** `text/event-stream`) for DataFusion:
//! `query_sql`, `get_schema`, `column_statistics` over a registered CSV.

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::routing::post;
use axum::{Router, response::IntoResponse};
use clap::Parser;
use datafusion::arrow::datatypes::Schema;
use datafusion::arrow::util::pretty::pretty_format_batches;
use datafusion::prelude::*;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

const MCP_SESSION_HEADER: &str = "mcp-session-id";
const ACCEPT_STREAMABLE: &str = "application/json, text/event-stream";

#[derive(Parser, Debug)]
#[command(name = "kowalski-mcp-datafusion")]
#[command(about = "MCP Streamable HTTP: SQL, schema, and column stats (DataFusion + CSV)")]
struct Args {
    /// Listen address (POST JSON-RPC to this URL — use as `[[mcp.servers]] url` in config.toml)
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: String,
    /// CSV file to register
    #[arg(long)]
    csv: PathBuf,
    /// SQL table name
    #[arg(long, default_value = "data")]
    table: String,
}

#[derive(Clone)]
struct AppState {
    ctx: Arc<SessionContext>,
    table: String,
    session_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !args.csv.exists() {
        return Err(format!("CSV not found: {}", args.csv.display()).into());
    }

    let ctx = SessionContext::new();
    let path = args
        .csv
        .to_str()
        .ok_or("CSV path must be valid UTF-8")?;
    ctx.register_csv(&args.table, path, CsvReadOptions::new())
        .await?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let state = AppState {
        ctx: Arc::new(ctx),
        table: args.table.clone(),
        session_id: session_id.clone(),
    };

    let app = Router::new()
        .route("/", post(mcp_post))
        .with_state(state);

    let addr: SocketAddr = args.bind.parse()?;
    eprintln!(
        "kowalski-mcp-datafusion: table `{}` <- `{}` | session {} | listening http://{}",
        args.table,
        args.csv.display(),
        session_id,
        addr
    );
    eprintln!("Accept header for clients: `{}`", ACCEPT_STREAMABLE);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Streamable HTTP: respond with SSE when the client advertises `text/event-stream` (typical with `application/json, text/event-stream`).
fn wants_sse(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase().contains("text/event-stream"))
        .unwrap_or(false)
}

/// Prefer SSE when the client lists `text/event-stream` before JSON (rough heuristic).
fn response_for_envelope(
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
