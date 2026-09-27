# kowalski-mcp-rookery

Standalone **MCP** server exposing the **Rookery** horde-builder primitives of
[`kowalski-core`](https://crates.io/crates/kowalski-core). Any MCP client — a Kowalski agent, the
CLI, or a desktop client — can build hordes without the Kowalski UI or HTTP API. Runs over
**stdio** or **stateless Streamable HTTP** (built on [`emperor-mcp`](https://crates.io/crates/emperor-mcp)).

## Design: LLM-free, the calling agent drives the interview

The server runs **no LLM**. The calling agent conducts the interview and assembles a draft; this
server validates, parses and materializes it. All logic delegates to `kowalski_core::rookery`, the
same functions the Kowalski HTTP API uses.

## Tools

| Tool | Input | Returns |
|------|-------|---------|
| `rookery_example_draft` | _(none)_ | `{ draft }` — a minimal valid linear draft (schema reference and starting template) |
| `rookery_validate_draft` | `{ draft }` | `{ ok, errors, draft }` — normalized draft and validation result |
| `rookery_parse_draft` | `{ text }` | `{ ok, draft }` or `{ ok:false, error }` — parse a fenced JSON/YAML draft from assistant text |
| `rookery_give_birth` | `{ draft, output_root?, overwrite? }` | `{ ok, horde_id, horde_root, validate_ok, validate_errors }` — writes `horde.md`, `agents/`, `prompts/`, `README.md`, `AGENTS.md` and validates the tree |

Drafts may declare `edges[]` for fork/join hordes; linear hordes omit them.

## Run

```bash
cargo install kowalski-mcp-rookery
# stdio (default) — stdout is the protocol stream, logs go to stderr
kowalski-mcp-rookery --transport stdio --output-root hordes
# stateless Streamable HTTP
kowalski-mcp-rookery --transport http --bind 127.0.0.1:8081 --output-root hordes
```

`--output-root` is the give-birth default; the tool's `output_root` argument overrides it per call.

### Wire into Kowalski (`config.toml`)

```toml
[[mcp.servers]]
name = "rookery"
transport = "stdio"
command = ["kowalski-mcp-rookery", "--output-root", "hordes"]
```

or, with the server running over HTTP:

```toml
[[mcp.servers]]
name = "rookery"
transport = "http"
url = "http://127.0.0.1:8081/"
```

## License

MIT — see [LICENSE](https://github.com/yarenty/kowalski/blob/main/LICENSE).
