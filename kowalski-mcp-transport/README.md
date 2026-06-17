# kowalski-mcp-transport

**Version 1.4.0** — reusable **MCP transports** for in-repo Kowalski servers (tool *source 1*): **stdio** and **stateless Streamable HTTP**, sharing one handler.

A server implements the [`McpHandler`] trait (turn one JSON-RPC request into an optional reply; `None` for notifications) and picks a transport. No dispatch logic is duplicated across transports.

## Why

Every in-repo MCP server (`kowalski-mcp-rookery`, `kowalski-mcp-datafusion`, future ones) should be runnable as **stateless Streamable HTTP** — every POST is independent, no `Mcp-Session-Id` is issued or required, so the server is trivially restartable and horizontally scalable. This crate provides that once.

## API

```rust
use kowalski_mcp_transport::{McpHandler, run_stdio, serve_http};
use serde_json::{json, Value};
use std::sync::Arc;

struct MyServer;
impl McpHandler for MyServer {
    async fn handle(&self, request: Value) -> Option<Value> {
        let id = request.get("id")?.clone(); // None => notification, no reply
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} }))
    }
}

// stdio:           run_stdio(Arc::new(MyServer)).await?;
// stateless HTTP:  serve_http("127.0.0.1:8080".parse()?, Arc::new(MyServer)).await?;
```

- `run_stdio` — newline-delimited JSON-RPC on stdin/stdout. **Logs must go to stderr.**
- `serve_http` / `http_router` — stateless Streamable HTTP (`POST /`). Replies `application/json`, or a one-shot SSE `data:` frame when the client sends `Accept: text/event-stream` (`ACCEPT_STREAMABLE`). Notifications return `202 Accepted`.

## Tests

```bash
cargo test -p kowalski-mcp-transport
```

## See also

- [`AGENTS.md`](./AGENTS.md)
- Consumers: [`../kowalski-mcp-rookery/`](../kowalski-mcp-rookery/), [`../kowalski-mcp-datafusion/`](../kowalski-mcp-datafusion/)
- [`../kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md) — Tool execution model (sources)
