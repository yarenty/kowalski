# Manual & end-to-end verification

Automated checks live in **CI** (`.github/workflows/ci.yml`) and **`cargo test`**. This file lists **operator-only** and **live-environment** steps that are not fully automated.

---

## WP2 — MCP

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP2-M1 | **Real HTTP MCP server** | Point `[[mcp.servers]]` at a live endpoint (e.g. `kowalski-mcp-datafusion` via Docker + `fixtures/sample.csv`). Run `mcp ping` / `mcp tools`; confirm `tools/list` and `tools/call` succeed. |
| WP2-M2 | **Full tool loop with live LLM** | With Ollama (or compatible) running: prompt so the model emits **parseable tool JSON** → tool executes (built-in or MCP) → follow-up reply includes tool result. Depends on model following the schema. |
| WP2-M3 | **SSE MCP server** (optional) | When testing against a server that requires long-lived SSE session behavior beyond the mock, run `mcp ping` / chat with tools and confirm session stability. |

---

## WP3 — Postgres / memory / graph

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP3-M1 | **Database init & migrations** | Run Postgres (local or Docker) with extensions you need (`vector`, optionally **AGE**). `cargo run -p kowalski-cli --features postgres -- db migrate --url postgres://…` (or `-c config.toml`). Inspect tables / extensions. |
| WP3-M2 | **Episodic tier on Postgres** | With `memory.database_url` set to Postgres: exercise chat, then query `episodic_kv` (or relevant tables) for expected rows; optional **restart CLI** and confirm recall if persistence path is enabled. |
| WP3-M3 | **pgvector similarity** | With semantic memory on Postgres + embeddings: run a retrieval turn and confirm sensible rows (no strict CI assertion yet). |
| WP3-M4 | **Apache AGE / Cypher** | With **`kowalski-cli --features postgres`**, AGE installed, graph created: call **`POST /api/graph/cypher`** or use `kowalski_core::postgres_age_cypher` — Cypher must **`RETURN … AS result`**. CI uses `apache/age` image for one integration test; production DBs need manual smoke. |
| WP3-M5 | **Restart recalls memory** | Tier-2/3 persistence configured: send messages, restart `serve`, new session or same config — confirm expected memory behavior (manual; semantics depend on config). |

---

## WP4 — LLM & tool JSON

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP4-M1 | **Ollama end-to-end** | `ollama serve`, model pulled. `kowalski-cli serve` + UI or `POST /api/chat` — expect a normal reply. |
| WP4-M2 | **OpenAI-compatible end-to-end** | `[llm] provider = "openai"`, `openai_api_base` (LM Studio, Groq, vLLM, …), optional key; `[ollama].model` as model id. Verify via `serve` + `/api/chat` or CLI `chat`. |
| WP4-M3 | **JSON parser under load** | Feed **mangled** tool-shaped responses (fences, partial JSON) through **`chat_with_tools`** with MCP or built-in tools; confirm repair / self-correction path without panic. |
| WP4-M4 | **Self-correction loop** | Provoke invalid tool JSON once; confirm hint / next turn recovers (see `utils/json` + agent loop). |

---

## WP5 — Federation

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP5-M1 | **Postgres NOTIFY bridge** | `cargo run -p kowalski-cli --features postgres -- federation ping-notify -c config.toml` with valid `memory.database_url`; confirm NOTIFY path (see CLI help). |
| WP5-M2 | **Cross-process envelope** (optional) | Two processes / manual `pg_notify` to `kowalski_federation` with JSON **`AclEnvelope`** under size limits; SSE/WebSocket subscribers see events. |
| WP5-M3 | **Delegate path** | `POST /api/federation/delegate` with registry populated; confirm `delegated_to` and broker traffic. |

**Not implemented yet (product gaps, not a smoke checklist):** persistent **`AgentState`** table + heartbeats; **scored** capability routing (vs first match). Track in [`rebuild_tasks/wp5_federation_core_checks.md`](rebuild_tasks/wp5_federation_core_checks.md).

---

## WP6 — CLI, HTTP, Vue

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP6-M1 | **`kowalski run` REPL** | `cargo run -p kowalski-cli -- run -c config.toml` — orchestrator-style loop; chat and federation hints per `run_ops.rs`. |
| WP6-M2 | **SSE chat + token stream** | `POST /api/chat/stream` with default body: token deltas; with **`tools_stream`: true**, tokens only after tool rounds (if tools fire). |
| WP6-M3 | **Vue Chat** | `cd ui && bun run dev`; exercise Send, Send (SSE), **Tool-aware stream** checkbox, New conversation. |
| WP6-M4 | **Federation WebSocket** | `GET /api/federation/ws` (e.g. `websocat` or browser); events match broker. |
| WP6-M5 | **Graph tab** | `GET /api/graph/status`; with Postgres + AGE, exercise **`POST /api/graph/cypher`** from client or `curl`. |
| WP6-M6 | **Optional REPL UX** | Terminal prefixes (`[agent]`, `[tool]`) and long-stream ergonomics — if desired later. |

---

## Tests & CI
- [ ] **Episodic memory integration test** on real Postgres (insert/order by session) — called out in old WP3 text; not default CI.
- [ ] **pgvector cosine** integration test on live Postgres (optional; beyond current smoke).
- [ ] **Broader tool-JSON contract tests** — [`kowalski-core/ROADMAP.md`](kowalski-core/ROADMAP.md) (mock LLM / edge cases).

---

## DataFusion MCP (`kowalski-mcp-datafusion`)
- [ ] **Multi-table / Parquet** registration via CLI flags.
- [ ] **Performance / large-file** documentation (streaming vs load).
- [ ] **Ballista / distributed** (long shot; same MCP surface).

---

## Optional global smoke

- `cargo build --workspace && cargo test --workspace` — before a release.
- `cd ui && bun run build` — UI bundle.
- `cargo test -p kowalski-core --features postgres` with **`DATABASE_URL`** set — local Postgres.
- `cargo test -p kowalski-core --features postgres age_cypher` — requires DB with **Apache AGE** (or CI `postgres-age` job).

---

Not exhaustive; use ROADMAP for full checkboxes.

| Theme | Examples |
|-------|----------|
| **Memory** | Long-term conversation storage, search/indexing, context window management |
| **Tools** | More formats (DOCX, EPUB, HTML), OCR, tables, academic extras |
| **Template** | More domain templates, dynamic plugins |
| **Federation** | Protocol choice (A2A, ACP, …), **auth**, federation-wide logging/monitoring |
| **Agents** | More specialized agents / industry templates |
| **UI & integrations** | Export chats (PDF/HTML/MD), Slack/Discord/Teams |
| **Security** | E2E encryption, RBAC, anonymization, audit, filtering |
| **Analytics** | Usage, performance, cost, quality, error analytics |
| **Advanced** | i18n, prompt templates, CoT viz, semantic search across chats, auto-summary |
| **Dev** | Custom training tools, richer testing utilities |

