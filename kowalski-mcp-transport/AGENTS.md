# kowalski-mcp-transport — AI agent notes

**Crate**: `kowalski-mcp-transport` · **Version**: **1.2.0**

## Scope

Shared **transport layer** for in-repo MCP servers (tool *source 1*). Provides one [`McpHandler`] trait and two runners — **stdio** and **stateless Streamable HTTP** — so each server writes its tool dispatch **once** and gets both transports. Used by [`kowalski-mcp-rookery`](../kowalski-mcp-rookery/) and [`kowalski-mcp-datafusion`](../kowalski-mcp-datafusion/).

## Hard rules

- **Stateless HTTP only.** Never issue or require `Mcp-Session-Id`; every POST is independent. This keeps servers restartable / horizontally scalable and matches the Kowalski MCP client (`kowalski-core/src/mcp/client.rs`), which captures a session id *only if present* and never depends on one.
- **stdout is the protocol stream** under stdio — servers must log to **stderr**.
- **Notifications get no reply.** `McpHandler::handle` returns `None` for id-less messages; the HTTP transport then returns `202 Accepted` with an empty body, and stdio writes nothing.
- **Keep deps light.** Only `axum`, `tokio`, `serde_json`, `log` (all workspace). Do not pull tool-specific or heavy deps here.

## Before you change code

1. Read [`src/lib.rs`](./src/lib.rs) — `McpHandler`, `run_stdio`, `http_router`/`serve_http`, `json_or_sse`.
2. Run **`cargo test -p kowalski-mcp-transport`** (HTTP json/SSE/202/parse-error cases).
3. If you change the wire contract, re-run consumer tests: `cargo test -p kowalski-mcp-rookery -p kowalski-mcp-datafusion`.

## Documentation closure (mandatory)

Update [`README.md`](./README.md), consumer READMEs/AGENTS if the API changes, and root [`../CHANGELOG.md`](../CHANGELOG.md) when user-visible (root **Rule 7**).

## Related docs

- Root [`../AGENTS.md`](../AGENTS.md)
- [`../kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md) — Tool execution model
