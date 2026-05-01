# Kowalski UI roadmap

**Package version 1.1.0** (`package.json`). Depends on **`kowalski`** for `/api/*`.

## Near term

- [ ] Document production deploy patterns (`VITE_API_BASE`, static hosting behind reverse proxy) — see also [`DEPLOY.md`](./DEPLOY.md).

## Medium term

- [ ] Light UX polish for very large JSON payloads in debug / federation views.

## Done (1.1.0)

- [x] Federation panel: **horde-style run observability** — chat-like prompt for Knowledge Compiler runs, **task progress** timeline (`task_progress` events), and clearer final artifact / outcome context.
- [x] Tabs: Home, MCP, Chat (including **Tool-aware stream** / `tools_stream`), Federation, Graph status.
- [x] Vite proxy to local **`kowalski`** on port **3456**.

## Done (1.0.0)

- [x] Baseline operator shell for API-backed workflows.
