# kowalski-cli roadmap

Crate version **1.0.0** (see `Cargo.toml`). Workspace overview: **[`../ROADMAP.md`](../ROADMAP.md)**.

## Near term
- [ ] Packaging notes (static binary, systemd, container) without bloating the default binary.
- [ ] `serve`: optional TLS / bind address flags as needed for deployments.

## Medium term
- [ ] More operator diagnostics (config diff, MCP session introspection).

## Done (1.0.0 baseline)
- [x] `serve` with `/api/chat`, `/api/chat/stream` and **`tools_stream`** for tool-aware SSE.
- [x] `config`, `db migrate`, `doctor`, `mcp ping`, `mcp tools`, `run` REPL.
- [x] `--features postgres`: graph status + **`POST /api/graph/cypher`** when memory URL uses Postgres.
