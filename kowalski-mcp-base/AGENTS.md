# kowalski-mcp-base — AI agent notes

**Crate**: `kowalski-mcp-base` · **Version**: **1.6.0**

## Scope

Single framework crate for all **first-party** Kowalski MCP servers. Replaces `kowalski-mcp-transport`; adds output framing, forwarded headers, and rmcp `serve` bootstrap.

## Modules

| Module | Use |
|--------|-----|
| `transport` | `McpHandler`, `run_stdio`, `serve_http`, `http_router` |
| `serve` | rmcp `ServerHandler` bootstrap (`/mcp`, `/health`) |
| `framing` | `FrameKind`, `frame`, `structured_framed` |
| `headers` | `ForwardConfig`, `ForwardedHeaders`, middleware |

## Before you change code

1. Read [`MCP_REQUIREMENTS.md`](./MCP_REQUIREMENTS.md) and [`MANIFEST_SPEC.md`](./MANIFEST_SPEC.md).
2. Run **`cargo test -p kowalski-mcp-base`**.
3. Re-run consumer tests: **`cargo test -p kowalski-mcp-rookery -p kowalski-mcp-datafusion`**.

## Hard rules

- **Stateless HTTP** — no mandatory `Mcp-Session-Id`.
- **Framing at source** — every tool returns framed text via `FrameKind`.
- **No auth shortcuts** — no dev-mode credential bypasses.
- **Do not fork transport** in server crates — extend here.

## Documentation closure

Update this README, [`MCP_REQUIREMENTS.md`](./MCP_REQUIREMENTS.md), consumer AGENTS.md files, and root [`CHANGELOG.md`](../CHANGELOG.md) when behavior changes.

## Related

- [`../kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md) — Tool execution model
