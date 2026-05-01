# kowalski-cli roadmap

Crate version **1.1.0** (see `Cargo.toml`). Workspace overview: **[`../ROADMAP.md`](../ROADMAP.md)**.

## Near term

- [ ] Polish long JSON / federation log readability in the terminal (truncate, filters, or pager hooks).

## Medium term

- [ ] Desktop / shell wrappers (optional).

## Done (1.1.0)

- [x] Operator commands: `config`, `db migrate`, `doctor`, `mcp ping`, `mcp tools`, `run` REPL (`TemplateAgent` + tools from config).
- [x] Generic **`extension list`** / **`extension run`** dispatch for workspace extensions (e.g. Knowledge Compiler).
- [x] **`agent-app`** operators for markdown-defined orchestration (list / validate / run / delegate / worker / proof) aligned with **`examples/knowledge-compiler`**.
- [x] Federation-oriented CLI flows for **delegate** and **worker** apps publishing task outcomes (paired with **`kowalski`** HTTP API).

## Done (1.0.0 baseline)

- [x] Interactive / legacy orchestrator entry points (`--interactive`, `create`, `chat` by agent name) where still exposed.

## HTTP API note

The **`kowalski`** binary (crate **`kowalski`**) serves **`/api/*`** — chat, stream, MCP, federation, graph. Bind/TLS, `/api/doctor`, Postgres-backed federation registry features belong to that crate; see **[`../kowalski/README.md`](../kowalski/README.md)** and root **[`README.md`](../README.md)**.
