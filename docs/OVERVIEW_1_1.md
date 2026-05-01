# Kowalski 1.1.x overview

Release **1.1.0** centers on **horde-style app workflows**: multi-step, markdown-defined agent orchestration on top of the existing **`TemplateAgent`**, **HTTP API** (`kowalski` binary), **federation** primitives, and the **Vue** operator UI.

## What shipped vs 1.0.0

- **Knowledge Compiler** (`examples/knowledge-compiler`): local-first pipeline from web/source ingest to Obsidian-style **`wiki/`** artifacts, plus ask/lint passes.
- **Markdown-defined agents**: `main-agent.md` and `agents/*.md` describe orchestration and specialists; the CLI validates definitions before running.
- **Extension model**: `kowalski-cli extension list` / `extension run <name>` dispatches to workspace extensions (e.g. Knowledge Compiler runner).
- **`agent-app` operators**: list, validate, run, delegate, worker, proof — glue between markdown orchestration, **`POST /api/chat`**, and federation APIs.
- **Federation UX**: delegate/worker flows publish **task progress** and **task results** so operators see steps and artifact paths (CLI traces + Vue federation panel).

## Where to read more

| Topic | Doc |
|-------|-----|
| Operator setup & binaries | [`README.md`](../README.md) |
| Release notes | [`CHANGELOG.md`](../CHANGELOG.md) |
| Roadmap & checklists | [`ROADMAP.md`](../ROADMAP.md) |
| Knowledge Compiler | [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md) |
| Memory design goals | [`DESIGN_MEMORY_AND_DEPENDENCIES.md`](./DESIGN_MEMORY_AND_DEPENDENCIES.md) |

## Architecture reminder

- **`kowalski-core`**: `TemplateAgent`, tools, memory, MCP client, federation types.
- **`kowalski-cli`**: REPL, config/db/MCP operators, extensions, `agent-app`.
- **`kowalski`**: HTTP server for **`/api/*`** (chat, stream, MCP, federation, graph when built with Postgres).
- **`ui/`**: Vue shell calling the HTTP API (proxied in dev).
- **`kowalski-mcp-datafusion`**: optional standalone MCP server for DataFusion/SQL over files.

For historical articles that predate this layout, see [`purgatory/README.md`](./purgatory/README.md).
