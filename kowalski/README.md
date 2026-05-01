# Kowalski (facade crate)

Rust workspace crate that re-exports **[`kowalski-core`](../kowalski-core/README.md)** and optional **[`kowalski-cli`](../kowalski-cli/README.md)** so dependents can use one package name. Business logic lives in **`kowalski-core`** (`TemplateAgent`, tools, memory, MCP, federation).

## Version

**Crate version 1.1.0** (see workspace [`Cargo.toml`](../Cargo.toml)).

## Features

| Feature | Effect |
|---------|--------|
| *(default)* | `kowalski-core` only, re-exported as `kowalski::core` plus convenience `pub use` entries ([`src/lib.rs`](src/lib.rs)). |
| `cli` | Pulls in **`kowalski-cli`** as `kowalski::cli`. |
| `postgres` | Enables **`kowalski-core/postgres`** (SQL memory, pgvector helpers). |
| `full` | `cli` + `postgres`. |

There are **no** separate `kowalski-academic-agent`, `kowalski-tools`, or `kowalski-web-agent` crates in this repository—compose behavior with **`TemplateAgent`**, configuration, and tools.

## Binary: HTTP API

This crate builds the **`kowalski`** executable (**`/api/*`** for the Vue UI and integrations). See the root **[README.md](../README.md)** for run instructions (`cargo run -p kowalski`).

## Usage (`Cargo.toml`)

```toml
[dependencies]
kowalski = "1.1.0"

# Optional: CLI + Postgres-capable core
kowalski = { version = "1.1.0", features = ["full"] }
```

## Example (Rust API)

```rust
use kowalski::{Config, TemplateAgent};

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
- [Workspace README](../README.md) · [AGENTS.md](./AGENTS.md) · [CONTRIBUTING.md](../CONTRIBUTING.md)

## License

MIT — see [LICENSE](../LICENSE).
