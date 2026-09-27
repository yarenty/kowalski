# kowalski-cli

Command line of **Kowalski**, a Rust multi-agent framework: agents with MCP tools and memory, and
hordes (step pipelines with triggers) over Ollama or OpenAI-compatible models.

The operator UI and HTTP API are served by the [`kowalski`](https://crates.io/crates/kowalski)
binary; `kowalski-cli` is the terminal side.

## Install

```bash
cargo install kowalski-cli
kowalski-cli --help
```

## Commands

- `run` — interactive agent REPL
- `config check` — validate a config file
- `db migrate` — memory database migrations
- `doctor` — health diagnostics
- `mcp ping`, `mcp tools` — check configured MCP servers
- `extension list`, `extension run` — discover and run extensions
- `federation ping-notify` — federation smoke test (with `--features postgres`)

```bash
kowalski-cli run -c config.toml
kowalski-cli doctor
kowalski-cli mcp tools -c config.toml
```

## Extensions

`extension run <name>` resolves, in order:

1. a binary in `PATH` named `kowalski-ext-<name>`
2. a local executable `.kowalski/extensions/<name>/run`

See the [Knowledge Compiler example](https://github.com/yarenty/kowalski/tree/main/examples/knowledge-compiler) for a
delegate/worker flow against a running `kowalski` server.

## Documentation

- [docs.rs/kowalski-cli](https://docs.rs/kowalski-cli)
- [Project README](https://github.com/yarenty/kowalski#readme) · [Changelog](https://github.com/yarenty/kowalski/blob/main/CHANGELOG.md)

## License

MIT — see [LICENSE](https://github.com/yarenty/kowalski/blob/main/LICENSE).
