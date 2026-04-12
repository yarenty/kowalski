# kowalski-cli roadmap

Crate version **1.0.0** (see `Cargo.toml`). Workspace overview: **[`../ROADMAP.md`](../ROADMAP.md)**.

## Near term
- [ ] UX polish on long JSON / federation logs in the Vue UI.

## Medium term
- [ ] Desktop / shell wrappers (optional).

## Done (1.0.0 baseline)
- [x] `serve` with `/api/chat`, `/api/chat/stream` and **`tools_stream`** for tool-aware SSE.
- [x] `config`, `db migrate`, `doctor`, `mcp ping`, `mcp tools`, `run` REPL.
- [x] `--features postgres`: graph status + **`POST /api/graph/cypher`** when memory URL uses Postgres.
- [x] `serve --bind`: listen address; **`--tls-cert` / `--tls-key`** HTTPS (rustls).
- [x] Operator docs: [`PACKAGING.md`](PACKAGING.md); `/api/doctor` includes `operator` (config divergence vs defaults, MCP count, Postgres flag, MCP session note).
- [x] Postgres: **`agent_state`** upsert with registry load; **`POST /api/federation/heartbeat`**; registry JSON merges **`state`** rows when available.
