# kowalski-mcp-datafusion — AI agent notes

**Crate**: `kowalski-mcp-datafusion` · **Version**: **1.4.0**

## Scope

This crate is a **standalone MCP HTTP server** (not the main `kowalski` agent). It implements the DataFusion tool dispatch as an [`McpHandler`] and serves it over the shared [`kowalski-mcp-transport`](../kowalski-mcp-transport/) as **stateless Streamable HTTP** (JSON/SSE, **no `Mcp-Session-Id`**), consistent with `kowalski-core`'s MCP client.

## Before you change code

1. Read [`src/lib.rs`](./src/lib.rs) for the MCP request/response flow and tool handlers.
2. Run **`cargo test -p kowalski-mcp-datafusion`** (includes HTTP smoke tests).
3. If changing the Docker image, rebuild with **`docker compose -f kowalski-mcp-datafusion/docker-compose.yml build`**.

## Conventions

- Keep **DataFusion** and heavy deps **only** in this crate’s `Cargo.toml` (not the workspace root).
- Prefer small, testable pure functions for SQL/schema helpers; keep the `McpHandler` dispatch thin.
- **Transport is shared + stateless.** HTTP/SSE/stdio framing lives in `kowalski-mcp-transport`; don't reimplement it here. The server must stay stateless (no session id).
- **Docker build context = repo root.** The `Dockerfile` `COPY`s each workspace member individually, so when a new member is added to the root `Cargo.toml`, add a matching `COPY` line (cargo loads the whole workspace even for `-p`).

## Documentation closure (mandatory)

After any refactor or behavior change in this crate, update **[`README.md`](./README.md)**, **[`ROADMAP.md`](./ROADMAP.md)**, root **[`CHANGELOG.md`](../CHANGELOG.md)** when user-visible, and **[`../docs/`](../docs/README.md)** if MCP/DataFusion architecture changes. **Shipping code without updating docs is incomplete work.**

## Related docs

- [`README.md`](./README.md)  
- [`ROADMAP.md`](./ROADMAP.md)  
- Root [`../AGENTS.md`](../AGENTS.md)
