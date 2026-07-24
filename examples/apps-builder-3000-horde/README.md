# Apps Builder 3000 Horde

A linear pipeline to ingest a project name and generate a simple Rust 'Hello World' application structure.

## Quick start

```bash
cargo run -p kowalski-cli -- agent-app validate --path .
cargo run -p kowalski-cli -- agent-app run --path . "your source text or URL"
```

Artifacts default to **`output/`** (see `horde.md`).

Born with **Rookery** (Kowalski 1.3.0).
