# Kowalski UI (Vue 3 + Vite)

Operator-facing web shell for Kowalski. The **CLI** (`kowalski-cli`) remains the primary control plane; this app is the starting point for dashboards (MCP health, chat, federation) once an HTTP API is available.

Use **[Bun](https://bun.sh)** for installs and scripts (not npm).

## Setup

```bash
cd ui
bun install
bun run dev
```

Open http://localhost:5173

## Build

```bash
bun run build
```

Static output is written to `dist/` (suitable for any static host or reverse proxy).

## Backend (CLI HTTP API)

In one terminal from the repo root:

```bash
cargo run -p kowalski-cli -- serve -c config.toml
```

This binds **`127.0.0.1:3000`** and serves JSON under `/api` (`/api/health`, `/api/doctor`, `/api/mcp/servers`, `POST /api/mcp/ping`, **`POST /api/chat`**, **`POST /api/chat/reset`** for a new conversation). Use `-c` / `--ollama-url` as needed (see `kowalski-cli serve --help`).

## API proxy

`vite.config.ts` proxies `/api` to `http://127.0.0.1:3000` so the Vue app can call relative paths like `/api/health`. For a production build on another origin, set `VITE_API_BASE` to the full API origin (no trailing slash).
