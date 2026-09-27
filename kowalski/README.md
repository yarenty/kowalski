# kowalski

The **Kowalski** server and facade crate. It builds the `kowalski` binary — one executable that serves
the operator UI, the `/api/*` HTTP API, built-in hordes and their triggers — and, as a library,
re-exports [`kowalski-core`](https://crates.io/crates/kowalski-core) (and optionally
[`kowalski-cli`](https://crates.io/crates/kowalski-cli)) under one package name.

Kowalski is a Rust multi-agent framework: agents with MCP tools and memory, and **hordes** (step
pipelines with cron, file-watch and webhook triggers) over Ollama or OpenAI-compatible models.

## Install the server

Pre-built binaries for macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash
kowalski    # opens the app in your browser; Setup writes the config
```

Or from crates.io: `cargo install kowalski`.

## Use as a library

```bash
cargo add kowalski                      # core only
cargo add kowalski --features full      # + CLI and Postgres-capable core
```

| Feature | Effect |
|---------|--------|
| *(default)* | `kowalski-core`, re-exported as `kowalski::core` plus common types at the crate root. |
| `cli` | `kowalski-cli` as `kowalski::cli`. |
| `postgres` | `kowalski-core/postgres` (SQL memory, pgvector, graph). |
| `full` | `cli` + `postgres`. |

```rust,no_run
use kowalski::{Agent, Config, TemplateAgent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let model = config.ollama.model.clone();
    let mut agent = TemplateAgent::new(config).await?;
    let conv = agent.start_conversation(&model);
    let reply = agent.chat_with_history(&conv, "Hello", None).await?;
    println!("{reply}");
    Ok(())
}
```

## Documentation

- [docs.rs/kowalski](https://docs.rs/kowalski)
- [Project README](https://github.com/yarenty/kowalski#readme) · [Architecture](https://github.com/yarenty/kowalski/blob/main/docs/architecture.html) · [Changelog](https://github.com/yarenty/kowalski/blob/main/CHANGELOG.md)
- Blog: [kowalski.yarenty.com](https://kowalski.yarenty.com)

## License

MIT — see [LICENSE](https://github.com/yarenty/kowalski/blob/main/LICENSE).
