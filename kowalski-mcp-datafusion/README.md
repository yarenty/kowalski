# kowalski-mcp-datafusion

**Version 1.0.0** — standalone **MCP** server (Streamable HTTP: JSON + SSE) exposing **DataFusion** tools over a registered **CSV** (or similar) table.

## Features

- **Streamable HTTP** with `Accept: application/json, text/event-stream`, **`Mcp-Session-Id`**.
- Tools: **`query_sql`**, **`get_schema`**, **`column_statistics`** (see `src/lib.rs`).
- **Docker**: `Dockerfile` and `docker-compose.yml` at repo paths under this crate.

## Run (dev)

```bash
cargo run -p kowalski-mcp-datafusion -- --help
```

## Tests

```bash
cargo test -p kowalski-mcp-datafusion
```

## See also

- [`AGENTS.md`](./AGENTS.md) — agent / contributor notes for this crate.
- [`ROADMAP.md`](./ROADMAP.md) — follow-ups specific to the DataFusion MCP server.
