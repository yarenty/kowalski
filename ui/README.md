# Kowalski UI (Vue 3 + Vite)

**Version 1.1.0** · Operator-facing web shell for Kowalski, calling **`kowalski`** under `/api/*`.

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
