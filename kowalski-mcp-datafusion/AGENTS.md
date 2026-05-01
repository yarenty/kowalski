# kowalski-mcp-datafusion — AI agent notes

**Crate**: `kowalski-mcp-datafusion` · **Version**: **1.1.0**

## Scope

This crate is a **standalone MCP HTTP server** (not the main `kowalski` agent). It uses **Axum**, **DataFusion**, and implements MCP **Streamable HTTP** patterns consistent with `kowalski-core`’s MCP client (session header, JSON/SSE bodies).

## Before you change code

1. Read [`src/lib.rs`](./src/lib.rs) for the MCP request/response flow and tool handlers.
2. Run **`cargo test -p kowalski-mcp-datafusion`** (includes HTTP smoke tests).
3. If changing the Docker image, rebuild with **`docker compose -f kowalski-mcp-datafusion/docker-compose.yml build`**.

## Conventions

- Keep **DataFusion** and heavy deps **only** in this crate’s `Cargo.toml` (not the workspace root).
- Prefer small, testable pure functions for SQL/schema helpers; keep Axum handlers thin.

## Related docs

- [`README.md`](./README.md)  
- [`ROADMAP.md`](./ROADMAP.md)  
- Root [`../AGENTS.md`](../AGENTS.md)
