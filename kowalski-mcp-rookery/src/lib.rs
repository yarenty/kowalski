//! Stdio MCP server exposing **Rookery** horde-building primitives from `kowalski-core`.
//!
//! The server is intentionally **LLM-free**: the *calling* agent (Claude Desktop, the
//! Kowalski agent, or any MCP client) drives the interview and assembles a draft, then
//! uses these deterministic tools to validate, parse, and **give birth** to a horde tree.
//!
//! Tools (all delegate to `kowalski_core::rookery`):
//! - `rookery_example_draft` — return a minimal valid linear draft (teaches the schema).
//! - `rookery_validate_draft` — normalize + validate a draft; return `{ ok, errors, draft }`.
//! - `rookery_parse_draft` — parse a draft from assistant text (fenced JSON/YAML block).
//! - `rookery_give_birth` — write + validate a horde tree on disk; return paths.

use kowalski_core::rookery::{
    HordeBirthSpec, RookeryDraft, minimal_linear_draft, normalize_draft,
    parse_draft_from_assistant, validate_draft, validate_horde_tree, write_horde_tree,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// MCP protocol version reported on `initialize` (matches `kowalski-mcp-datafusion`).
pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Dispatch one JSON-RPC request value. Returns `None` for notifications (no reply expected).
///
/// `default_output_root` is used by `rookery_give_birth` when the call omits `output_root`.
pub fn dispatch(body: &Value, default_output_root: &Path) -> Option<Value> {
    // JSON-RPC notifications carry no `id` and must not be answered.
    let id = body.get("id")?.clone();
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {
                "name": "kowalski-mcp-rookery",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": { "tools": {} }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools_list_json()),
        "tools/call" => run_tool_call(body, default_output_root),
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

/// The advertised tool catalog (`tools/list`).
pub fn tools_list_json() -> Value {
    let draft_schema = json!({
        "type": "object",
        "description": "A Rookery draft. Call rookery_example_draft for the full shape.",
        "properties": {
            "id": { "type": "string", "description": "Horde id (slug; becomes the output directory name)" },
            "display_name": { "type": "string" },
            "description": { "type": "string" },
            "pipeline": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Ordered step names; must match penguins[].name exactly (linear only in 1.3.0)"
            },
            "penguins": {
                "type": "array",
                "description": "One entry per pipeline step",
                "items": { "type": "object" }
            }
        },
        "required": ["id", "display_name", "description", "pipeline", "penguins"]
    });

    json!({
        "tools": [
            {
                "name": "rookery_example_draft",
                "description": "Return a minimal, valid linear horde draft to use as a starting template / schema reference.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "rookery_validate_draft",
                "description": "Normalize and validate a Rookery draft. Returns { ok, errors, draft } with the normalized draft.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "draft": draft_schema },
                    "required": ["draft"]
                }
            },
            {
                "name": "rookery_parse_draft",
                "description": "Parse a Rookery draft from assistant text containing a fenced JSON or YAML block. Returns { ok, draft } or { ok:false, error }.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Assistant message text containing a fenced draft block" }
                    },
                    "required": ["text"]
                }
            },
            {
                "name": "rookery_give_birth",
                "description": "Write a horde tree to disk (agents/, prompts/, horde.md, README/AGENTS) and validate it. Returns { ok, horde_id, horde_root, validate_errors }.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "draft": json!({ "type": "object", "description": "Draft to materialize (same shape as rookery_validate_draft)" }),
                        "output_root": { "type": "string", "description": "Directory to write <id>/ into. Defaults to the server's --output-root." },
                        "overwrite": { "type": "boolean", "description": "Replace an existing <output_root>/<id>/ tree (default false)." }
                    },
                    "required": ["draft"]
                }
            }
        ]
    })
}

fn run_tool_call(body: &Value, default_output_root: &Path) -> Result<Value, String> {
    let params = body.get("params").cloned().unwrap_or_else(|| json!({}));
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "rookery_example_draft" => {
            let draft = minimal_linear_draft();
            Ok(json_result(json!({ "draft": draft })))
        }
        "rookery_validate_draft" => {
            let mut draft = parse_draft_arg(&args)?;
            normalize_draft(&mut draft);
            match validate_draft(&draft) {
                Ok(()) => Ok(json_result(
                    json!({ "ok": true, "errors": Value::Null, "draft": draft }),
                )),
                Err(e) => Ok(json_result(
                    json!({ "ok": false, "errors": e.to_string(), "draft": draft }),
                )),
            }
        }
        "rookery_parse_draft" => {
            let text = args
                .get("text")
                .and_then(|t| t.as_str())
                .ok_or_else(|| "missing arguments.text".to_string())?;
            match parse_draft_from_assistant(text) {
                Ok(mut draft) => {
                    normalize_draft(&mut draft);
                    Ok(json_result(json!({ "ok": true, "draft": draft })))
                }
                Err(e) => Ok(json_result(json!({ "ok": false, "error": e.to_string() }))),
            }
        }
        "rookery_give_birth" => {
            let mut draft = parse_draft_arg(&args)?;
            normalize_draft(&mut draft);
            validate_draft(&draft).map_err(|e| format!("invalid draft: {e}"))?;

            let root = args
                .get("output_root")
                .and_then(|s| s.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| default_output_root.to_path_buf());
            let overwrite = args
                .get("overwrite")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);

            let spec = HordeBirthSpec::new(draft.clone()).with_overwrite(overwrite);
            let horde_root = write_horde_tree(&root, &spec).map_err(|e| e.to_string())?;
            let (validate_ok, validate_errors) = match validate_horde_tree(&horde_root) {
                Ok(()) => (true, Value::Null),
                Err(e) => (false, Value::String(e.to_string())),
            };
            Ok(json_result(json!({
                "ok": validate_ok,
                "horde_id": draft.id,
                "horde_root": horde_root.display().to_string(),
                "validate_ok": validate_ok,
                "validate_errors": validate_errors,
            })))
        }
        other => Err(format!("unknown tool: {other}")),
    }
}

fn parse_draft_arg(args: &Value) -> Result<RookeryDraft, String> {
    let draft = args
        .get("draft")
        .ok_or_else(|| "missing arguments.draft".to_string())?;
    serde_json::from_value(draft.clone()).map_err(|e| format!("invalid draft: {e}"))
}

/// Wrap a structured value as an MCP `tools/call` result (pretty-printed text content).
fn json_result(v: Value) -> Value {
    let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
    json!({ "content": [{ "type": "text", "text": text }] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call(tool: &str, args: Value, root: &Path) -> Value {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": tool, "arguments": args }
        });
        dispatch(&req, root).expect("tools/call returns a reply")
    }

    fn result_payload(reply: &Value) -> Value {
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        serde_json::from_str(text).expect("payload is JSON")
    }

    #[test]
    fn initialize_advertises_tools() {
        let req = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" });
        let reply = dispatch(&req, Path::new(".")).unwrap();
        assert_eq!(
            reply["result"]["serverInfo"]["name"],
            "kowalski-mcp-rookery"
        );
        assert!(reply["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn notification_has_no_reply() {
        let req = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(dispatch(&req, Path::new(".")).is_none());
    }

    #[test]
    fn tools_list_has_four_tools() {
        let req = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" });
        let reply = dispatch(&req, Path::new(".")).unwrap();
        let tools = reply["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
    }

    #[test]
    fn example_then_validate_then_give_birth() {
        let dir = tempfile::tempdir().unwrap();

        let example = call("rookery_example_draft", json!({}), dir.path());
        let draft = result_payload(&example)["draft"].clone();

        let validated = call(
            "rookery_validate_draft",
            json!({ "draft": draft }),
            dir.path(),
        );
        assert_eq!(result_payload(&validated)["ok"], true);

        let born = call(
            "rookery_give_birth",
            json!({ "draft": draft, "output_root": dir.path().display().to_string() }),
            dir.path(),
        );
        let payload = result_payload(&born);
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["horde_id"], "rookery-demo");
        let root = payload["horde_root"].as_str().unwrap();
        assert!(std::path::Path::new(root).join("horde.md").is_file());
    }

    #[test]
    fn validate_reports_errors() {
        // A pipeline step with no matching penguin must fail validation.
        let mut broken = minimal_linear_draft();
        broken.pipeline.push("missing-step".into());
        let reply = call(
            "rookery_validate_draft",
            json!({ "draft": serde_json::to_value(&broken).unwrap() }),
            Path::new("."),
        );
        let payload = result_payload(&reply);
        assert_eq!(payload["ok"], false);
        assert!(payload["errors"].is_string());
    }
}
