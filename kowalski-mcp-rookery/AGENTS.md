# kowalski-mcp-rookery — AI agent notes

**Crate**: `kowalski-mcp-rookery` · **Version**: **1.2.0**

## Scope

Standalone **MCP server** that exposes the **Rookery** horde-builder primitives, over **stdio** or **stateless Streamable HTTP** (`--transport stdio|http`, shared [`../kowalski-mcp-transport`](../kowalski-mcp-transport/)). It is an **in-repo MCP server** (tool *source 1* per [`../kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md), *Tool execution model*) and the reposition described in [`../PLAN.md`](../PLAN.md) **§R2**: the builder is callable from CLI / external agents, not only the Vue tab.

## Hard rules

- **LLM-free.** This server runs no model. The *calling* agent drives the interview; this server only validates / parses / writes. Do not add an LLM dependency here.
- **No duplicated orchestration.** Every tool delegates to `kowalski_core::rookery` (`validate_draft`, `normalize_draft`, `parse_draft_from_assistant`, `write_horde_tree`, `validate_horde_tree`, `minimal_linear_draft`). The HTTP `/api/rookery/*` give-birth path calls the **same** core functions — keep it that way (R2.3).
- **stdout is the protocol stream** (stdio mode). Logs go to **stderr** only (`env_logger`). Never `println!` diagnostics.
- **Notifications get no reply.** JSON-RPC messages without an `id` (e.g. `notifications/initialized`) must not produce a response (`dispatch` returns `None`).
- **Transport is shared + stateless.** stdio and HTTP both run the same `RookeryHandler` (`dispatch`) via `kowalski-mcp-transport`. The HTTP transport is **stateless** (no `Mcp-Session-Id`). Don't fork transport logic here — extend the transport crate.

## Before you change code

1. Read [`src/lib.rs`](./src/lib.rs) — `dispatch` + `tools/list` + `run_tool_call`.
2. Run **`cargo test -p kowalski-mcp-rookery`** (covers initialize / tools/list / validate / give-birth round trip).
3. For an end-to-end check, pipe JSON-RPC lines into the binary (see [`README.md`](./README.md) → *Quick stdio smoke*).

## Adding a tool

- Implement the behavior in `kowalski-core::rookery` first (pure, testable), then expose it here as a thin wrapper.
- Register it in `tools_list_json()` **and** `run_tool_call()`; add a round-trip test.
- Tool results use MCP text content: `{ "content": [{ "type": "text", "text": "<pretty JSON>" }] }` (see `json_result`).

## Documentation closure (mandatory)

After any change, update **[`README.md`](./README.md)**, root **[`../CHANGELOG.md`](../CHANGELOG.md)** when user-visible, **[`../ROADMAP.md`](../ROADMAP.md)**, and the tool-source note in **[`../kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md)** if the surface changes. Shipping code without docs is incomplete work (root **Rule 7**).

## Related docs

- Root [`../AGENTS.md`](../AGENTS.md)
- [`../kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md) — Tool execution model
- [`../PLAN.md`](../PLAN.md) — §R2 reposition
