# kowalski-mcp-rookery

**Version 1.3.0** — standalone **MCP** server exposing the **Rookery** horde-builder primitives from `kowalski-core`. Runs over **stdio** *or* **stateless Streamable HTTP** (shared [`kowalski-mcp-transport`](../kowalski-mcp-transport/)).

This is an **in-repo MCP server** (tool *source 1* in [`kowalski-core/AGENTS.md`](../kowalski-core/AGENTS.md)). It lets any MCP client — the Kowalski agent, the CLI, or an external client such as Claude Desktop — build hordes ("penguins") without going through the Vue UI or the HTTP `/api/rookery/*` surface.

## Design: LLM-free, calling agent drives the interview

The server intentionally runs **no LLM**. The *calling* agent conducts the interview and assembles a draft; this server provides the **deterministic** primitives to validate, parse, and materialize that draft. All logic delegates to `kowalski_core::rookery` (the same functions the HTTP API uses), so there is no duplicated orchestration.

## Tools

| Tool | Input | Returns |
|------|-------|---------|
| `rookery_example_draft` | _(none)_ | `{ draft }` — a minimal valid linear draft (schema reference / starting template) |
| `rookery_validate_draft` | `{ draft }` | `{ ok, errors, draft }` — normalized draft + validation result |
| `rookery_parse_draft` | `{ text }` | `{ ok, draft }` or `{ ok:false, error }` — parse a fenced JSON/YAML draft block from assistant text |
| `rookery_give_birth` | `{ draft, output_root?, overwrite? }` | `{ ok, horde_id, horde_root, validate_ok, validate_errors }` — writes `agents/`, `prompts/`, `horde.md`, `README.md`, `AGENTS.md` and validates the tree |

> Linear pipelines only in 1.3.0 (`pipeline = [...]`). DAG / `edges[]` are 1.4.0+.

## Run (dev)

```bash
cargo run -p kowalski-mcp-rookery -- --help
# stdio (default) — for desktop MCP clients; stdout is the protocol stream, logs go to stderr:
cargo run -p kowalski-mcp-rookery -- --transport stdio --output-root examples
# stateless Streamable HTTP — no Mcp-Session-Id, every POST independent:
cargo run -p kowalski-mcp-rookery -- --transport http --bind 127.0.0.1:8081 --output-root examples
```

`--output-root` sets the give-birth default (override per-call via the tool's `output_root` arg).

### Quick stdio smoke

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rookery_example_draft","arguments":{}}}' \
  | cargo run -q -p kowalski-mcp-rookery -- --output-root /tmp/rookery-out
```

### Wire into Kowalski (`config.toml`)

stdio (subprocess launched by Kowalski):

```toml
[[mcp.servers]]
name = "rookery"
transport = "stdio"
command = ["cargo", "run", "-q", "-p", "kowalski-mcp-rookery", "--", "--output-root", "examples"]
```

or stateless HTTP (run the server separately, then point Kowalski at it):

```toml
[[mcp.servers]]
name = "rookery"
transport = "http"           # or "sse" — both use Streamable HTTP
url = "http://127.0.0.1:8081/"
```

Then: `cargo run -p kowalski-cli -- mcp ping` / `mcp tools`.

## Tests

```bash
cargo test -p kowalski-mcp-rookery
```

## See also

- [`AGENTS.md`](./AGENTS.md) — agent / contributor notes for this crate.
- [`../kowalski-core/src/rookery/`](../kowalski-core/src/rookery/) — the primitives this server wraps.
- [`../ROADMAP.md`](../ROADMAP.md) — R2 reposition rationale (1.3.x cleanup).
