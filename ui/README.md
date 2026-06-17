# Kowalski UI (Vue 3 + Vite)

**Version 1.4.0** · Operator-facing web shell for Kowalski, calling **`kowalski`** under `/api/*`.

Features: health, MCP ping, **Chat** (`POST /api/chat`, SSE **`POST /api/chat/stream`** with optional **Tool-aware stream** / `tools_stream`), federation, graph extension status. See [`ROADMAP.md`](./ROADMAP.md).

## Horde changes in 1.1.0 (since 1.0.0)

- Federation panel now supports clearer horde run observability with task progress events.
- Knowledge Compiler delegate/worker runs surface serialized step progress and final artifact delivery context in the UI flow.

## Setup

```bash
cd ui
bun install
bun run dev
```

Open [http://localhost:5173](http://localhost:5173)

Vite uses Rollup internally (transitive dependency). You do not need to add Rollup directly, but it still needs the matching native package for your runtime architecture.

If you hit `Cannot find module @rollup/rollup-darwin-*` on macOS, your runtime arch is usually mismatched (for example Rosetta x64 vs arm64). Use Node 22 arm64 and reinstall:

```bash
cd ui
rm -rf node_modules bun.lockb
bun install
```

## Build

```bash
bun run build
```

Static output is written to `dist/` (suitable for any static host or reverse proxy).

## Backend (HTTP API)

In one terminal from the repo root:

```bash
cargo run -p kowalski -- -c config.toml
```

This binds **`127.0.0.1:3456`** and serves JSON under `/api` (`/api/health`, `/api/doctor`, `/api/mcp/servers`, `POST /api/mcp/ping`, **`POST /api/chat`**, **`POST /api/chat/stream`** (body may include **`tools_stream`: true**), **`POST /api/chat/reset`**). With **`kowalski --features postgres`** and a Postgres memory URL, graph routes may include **`POST /api/graph/cypher`** (Apache AGE on the server). Use `-c` / `--ollama-url` as needed (see `kowalski --help`).

## API proxy

`vite.config.ts` proxies `/api` to `http://127.0.0.1:3456` so the Vue app can call relative paths like `/api/health`. For a production build on another origin, set `VITE_API_BASE` to the full API origin (no trailing slash).

## Operator smoke checklist (~2 minutes)

Use this after any change to **`kowalski`**, **`kowalski-core`**, or **`ui/`** that could affect `/api/*`, horde catalog, federation, or chat. Full governance: [`AGENTS.md`](./AGENTS.md) (**UI-first**).

**Prerequisites**

1. Repo root: `cargo run -p kowalski -- -c config.toml` (default `http://127.0.0.1:3456`).
2. Second terminal: `cd ui && bun install && bun run dev` → open [http://localhost:5173](http://localhost:5173).

**Steps**

| # | Sidebar tab | What to do | Pass criteria |
|---|-------------|------------|-----------------|
| 1 | **Home** | Open once | No blank crash; optional: app version from health appears when API is up. |
| 2 | **Chat** | Send one short message | **Optional** if `[llm]` / Ollama is configured: you get a normal reply or a **clear** error in the thread (not a silent hang). Skip if you have no LLM. |
| 3 | **Federation** | Scroll to **Knowledge Sucking Swarm** (Knowledge Compiler horde) → **Start All** | Workers move toward ready; no permanent red error. If workers never become ready, start matching `agent-app worker … --role …` processes from [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md). |
| 4 | **Horde** | **Knowledge Sucking Swarm**: URL + question form → **Run Horde**. **Rust Project Scaffolder** (`examples/rust-project-scaffolder`): operator form (project name, goals, crate shape) then **Run Horde** | Stream shows pipeline steps or explicit failure. Scaffolder ingest needs valid `output` paths (auto-repaired on birth/repair). |
| 5 | **Rookery** (1.3.0+) | **New session** → describe a 3-step workflow → **Propose horde** → **Give birth** | Summary + pipeline on the right (horizontal track, or layered **DAG** canvas when `edges[]` is present); birth shows path under `examples/<id>/`. Run `cargo run -p kowalski-cli -- agent-app validate --path examples/<id>` to confirm. Requires live LLM for chat/propose. |
| 5b | **Rookery / Horde** (DAG, optional) | Restart server; select **Coding assistant (planning)** in **Horde** | Canvas shows fork/join layers; operator form (project path + task); **Start All** workers then run. |
| 6 | **Federation** (optional extra) | Lower on the same panel: **Refresh registry** if you use raw delegate / `kc.run` smoke | Registry JSON loads; see [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md) for legacy worker commands. |



**Rust scaffolder demo** (Rookery → Horde):

1. **Rookery**: use the TEST PROMPT below → **Propose horde** → **Give birth** → `examples/rust-project-scaffolder/`
2. Restart `kowalski` (reload horde catalog) → **Horde** tab → select **Rust Project Scaffolder Pipeline**
3. Fill the **Operator input** form (project name, goals, optional URL, crate shape) → **Run horde**

If an older born horde still has `output = "String"`, call `POST /api/hordes/rust-project-scaffolder/repair-outputs` or re-birth from Rookery with overwrite.

TEST PROMPT (Rookery):
```txt
When having new rust project could you create pipeline to setup initial repository - project structure, invetigate crates that could be user for project, create first initial mock/mvp of the project and suggest todo list
```

**Failure triage**

- **CORS / network**: confirm Vite dev proxy and that the browser URL is the Vite origin (5173), not the API port directly.
- **Horde run stuck**: workers not started or wrong topic — return to step 3 and server logs.
- **Chat only**: horde can still be healthy; file issues separately if Chat breaks but Horde passes.

## See also

- [`AGENTS.md`](./AGENTS.md) — UI-first acceptance and conventions.
- [`../examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md) — CLI worker commands aligned with the UI.
