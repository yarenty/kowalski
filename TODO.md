# Operator QA, backlog, and release checks

**Automated:** CI (`.github/workflows/ci.yml`) and **`cargo test`**. This file is the **single live list** for manual/e2e verification, **product/engineering follow-ups**, and **repo hygiene**. Older planning notes may exist only in **git history**; do not treat stale external checklists as the source of truth.

**See also:** [`ROADMAP.md`](ROADMAP.md) · [`CHANGELOG.md`](CHANGELOG.md)

---

## Backlog — code, UX, docs

### Federation (`kowalski-core` / WP5)

| ID | Task | Notes |
|----|------|--------|
| FB-WP5-1 | **Automated multi-cluster lifecycle** | Optional: scheduled stale cleanup, auth on federation HTTP mutations, cross-process agent discovery — beyond current **`/api/federation/*`** + Postgres. |
| FB-WP5-2 | **Stricter ACL defaults** | Tighten `default_max_delegation_depth` / envelope validation defaults; document operator tuning. |
| FB-WP5-3 | **TTL / delegation loop hardening** | Optional watchdog for stale heartbeats; cross-check with [`kowalski-core/ROADMAP.md`](kowalski-core/ROADMAP.md) federation items. |

*Implemented baseline:* ranked capability routing, `agent_state` persistence, `POST /api/federation/heartbeat`, registry merge.

### UI (`ui/`)

| ID | Task | Notes |
|----|------|--------|
| FB-WP6-1 | **UX polish** for long JSON / federation logs | Collapsible panels, formatting, performance on large payloads. |
| FB-WP6-2 | **REPL-style prefixes** (`[agent]`, `[tool]`) | Terminal ergonomics; ties to WP6-M6. |
| FB-WP6-3 | **Federation timeline view** | Richer than raw SSE stream; optional. |
| FB-WP6-4 | **Graph explorer UI** | Beyond status + Cypher form; full explore TBD. |

### MCP / CLI

| ID | Task | Notes |
|----|------|--------|
| FB-WP2-1 | **Extra MCP transports / polish** | Stdio ergonomics, live e2e vs mocks — see **WP2** below and [`kowalski-core/ROADMAP.md`](kowalski-core/ROADMAP.md). |
| FB-WP2-2 | **Config / operator wizards** (optional) | Guided MCP server registration and validation (operator UX). |

### Tests & CI (extended)

| ID | Task | Notes |
|----|------|--------|
| FB-CI-1 | **Episodic memory integration test** on real Postgres | Insert/order by session — not default CI. |
| FB-CI-2 | **pgvector cosine** integration test | Optional; beyond current smoke. |
| FB-CI-3 | **Broader tool-JSON contract tests** | Mock LLM / edge cases — [`kowalski-core/ROADMAP.md`](kowalski-core/ROADMAP.md). |
| FB-CI-4 | **CI image or job with Apache AGE** | Automated Cypher tests without manual DB (optional follow-up). |

### DataFusion MCP (`kowalski-mcp-datafusion`)

| ID | Task | Notes |
|----|------|--------|
| FB-DF-1 | **Multi-table / Parquet** registration via CLI flags | |
| FB-DF-2 | **Performance / large-file** documentation | Streaming vs load. |
| FB-DF-3 | **Ballista / distributed** | Long shot; same MCP surface. |

### Documentation & repo hygiene

| ID | Task | Notes |
|----|------|--------|
| FB-DOC-1 | **Keep design docs aligned** with the shipped stack | e.g. [`docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](docs/DESIGN_MEMORY_AND_DEPENDENCIES.md); open decisions in [`ROADMAP.md`](ROADMAP.md). |
| FB-DOC-2 | **Cross-check crate `ROADMAP.md` files** | Avoid drift vs code and this backlog. |
| FB-DOC-3 | **Consolidated positioning narrative** | Optional deep pass vs OpenClaw-class tools — track themes in [`ROADMAP.md`](ROADMAP.md). |
| FB-REPO-1 | **History rewrite on GitHub** (optional) | Only if sensitive content removal is required — coordinate with maintainers. |

---

## WP2 — MCP (manual / e2e)

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
| WP3-M5 | **Restart recalls memory** | Tier-2/3 persistence configured: send messages, restart `kowalski`, new session or same config — confirm expected memory behavior (manual; semantics depend on config). |

---

## WP4 — LLM & tool JSON

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP4-M1 | **Ollama end-to-end** | `ollama serve`, model pulled. `kowalski` + UI or `POST /api/chat` — expect a normal reply. |
| WP4-M2 | **OpenAI-compatible end-to-end** | `[llm] provider = "openai"`, `openai_api_base` (LM Studio, Groq, vLLM, …), optional key; `[ollama].model` as model id. Verify via `kowalski` + `/api/chat` or CLI `chat`. |
| WP4-M3 | **JSON parser under load** | Feed **mangled** tool-shaped responses (fences, partial JSON) through **`chat_with_tools`** with MCP or built-in tools; confirm repair / self-correction path without panic. |
| WP4-M4 | **Self-correction loop** | Provoke invalid tool JSON once; confirm hint / next turn recovers (see `utils/json` + agent loop). |

---

## WP5 — Federation

| ID | What to verify | How (summary) |
|----|----------------|----------------|
| WP5-M1 | **Postgres NOTIFY bridge** | `cargo run -p kowalski-cli --features postgres -- federation ping-notify -c config.toml` with valid `memory.database_url`; confirm NOTIFY path (see CLI help). |
| WP5-M2 | **Cross-process envelope** (optional) | Two processes / manual `pg_notify` to `kowalski_federation` with JSON **`AclEnvelope`** under size limits; SSE/WebSocket subscribers see events. |
| WP5-M3 | **Delegate path** | `POST /api/federation/delegate` with registry populated; confirm `delegated_to` and broker traffic. |
| WP5-M4 | **`agent_state` + heartbeat** (Postgres) | With DB configured: `GET /api/federation/registry` includes merged **`state`**; **`POST /api/federation/heartbeat`** bumps `updated_at`. |

**Product gaps** (not smoke-only): see **Backlog — Federation** above (`FB-WP5-*`).

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

## Aspirational themes (not scheduled)

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
| **Edge / CPU** | ARM builds, Ollama quantized models, Raspberry Pi / edge deployment notes in `docs/` or crate READMEs |

Not exhaustive; see **ROADMAP** for crate-level checkboxes.
