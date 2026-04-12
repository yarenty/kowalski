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

## API proxy

`vite.config.ts` proxies `/api` to `http://127.0.0.1:3000` by default. Adjust the target when you add a Kowalski HTTP server.
