# kowalski-core

Core library of **Kowalski**, a Rust multi-agent framework: agents with MCP tools and memory, and
hordes (step pipelines with triggers) over Ollama or OpenAI-compatible models. The
[`kowalski`](https://crates.io/crates/kowalski) server and
[`kowalski-cli`](https://crates.io/crates/kowalski-cli) are thin surfaces over this crate.

```bash
cargo add kowalski-core
cargo add kowalski-core --features postgres   # SQL memory, pgvector, graph
```

## What is inside

| Area | Modules |
|------|---------|
| Agents | `agent` (`Agent` trait, `BaseAgent`, native tool calling with a ReAct fallback), `template` (`TemplateAgent`, `AgentBuilder`), `role` |
| Models | `llm` (Ollama and OpenAI-compatible providers, structured output), `model` |
| Tools | `tools` (`Tool` trait; internal `web_fetch`, `web_search`, filesystem), `tool_chain`, `mcp` (MCP client and hub: stdio and Streamable HTTP) |
| Memory | `memory` (working, episodic and semantic tiers), `db` (SQLite by default, Postgres optional; migrations ship with the crate) |
| Hordes | `horde_step`, `horde_stages`, `horde_graph` (linear and fork/join), `horde_trigger` (cron, watch, webhook), `horde_table_steps` (`table_profile`, `sql_batch`, `xlsx_report`), `markdown_pipeline`, `operator_input` |
| Building and sharing | `rookery` (horde builder primitives), `manifest` (portable workflow manifests and bundles), `source_bundle` |
| Federation | `federation` (agent registry, task delegation, events), `graph` |

## Example

```rust,no_run
use kowalski_core::{Agent, Config, template::TemplateAgent};

#[tokio::main]
async fn main() -> Result<(), kowalski_core::KowalskiError> {
    let config = Config::default();
    let model = config.ollama.model.clone();
    let mut agent = TemplateAgent::new(config).await?;
    let conv = agent.start_conversation(&model);
    println!("{}", agent.chat_with_history(&conv, "Hello", None).await?);
    Ok(())
}
```

## Documentation

- [docs.rs/kowalski-core](https://docs.rs/kowalski-core)
- [Architecture](https://github.com/yarenty/kowalski/blob/main/docs/architecture.html) · [Memory architecture](https://github.com/yarenty/kowalski/blob/main/kowalski-core/MEMORY_ARCHITECTURE.md) · [Changelog](https://github.com/yarenty/kowalski/blob/main/CHANGELOG.md)

## License

MIT — see [LICENSE](https://github.com/yarenty/kowalski/blob/main/LICENSE).
