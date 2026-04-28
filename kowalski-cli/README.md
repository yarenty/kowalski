# Kowalski CLI

**Crate version 1.0.0** · See [`ROADMAP.md`](./ROADMAP.md) and root [`README.md`](../README.md).

Command-line interface for Kowalski operators and extension workflows.

## Scope

`kowalski-cli` provides:

- TemplateAgent REPL (`run`)
- config checks (`config check`)
- memory DB migrations (`db migrate`)
- health diagnostics (`doctor`)
- MCP checks (`mcp ping`, `mcp tools`)
- federation smoke ops (`federation ping-notify`, with `--features postgres`)
- extension discovery and execution (`extension list`, `extension run`)

The HTTP API server for UI and federation routes is the separate `kowalski` binary.

## Quick start

```bash
# help
cargo run -p kowalski-cli -- --help

# interactive orchestrator REPL
cargo run -p kowalski-cli -- run -c config.toml

# diagnostics
cargo run -p kowalski-cli -- doctor
cargo run -p kowalski-cli -- config check config.toml

# MCP checks
cargo run -p kowalski-cli -- mcp ping -c config.toml
cargo run -p kowalski-cli -- mcp tools -c config.toml
```

## Extensions

```bash
# discover extensions
cargo run -p kowalski-cli -- extension list

# run an extension command
cargo run -p kowalski-cli -- extension run knowledge-compiler help
```

Extension resolution order:

1. Binary in `PATH` named `kowalski-ext-<name>`
2. Local executable `.kowalski/extensions/<name>/run`

## Federation-first app example

The first app example is the Knowledge Compiler extension:

- docs: [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md)
- local runner: `.kowalski/extensions/knowledge-compiler/run`

Typical flow:

```bash
# terminal 1: start HTTP API server
cargo run -p kowalski --bin kowalski

# terminal 2: start worker
cargo run -p kowalski-cli -- extension run knowledge-compiler worker kc-worker-1

# delegate tasks
cargo run -p kowalski-cli -- extension run knowledge-compiler delegate kc.compile "kc.compile"
```

## Notes

- Use `kowalski-cli` for operators and extension orchestration.
- Use `kowalski` for `/api/*` server routes.
