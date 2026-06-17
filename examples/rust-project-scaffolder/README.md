# Rust Project Scaffolder Pipeline

A pipeline designed to automate the setup of a new Rust project, including repository structure, dependency investigation, MVP creation, and task planning.

## Quick start

```bash
cargo run -p kowalski-cli -- agent-app validate --path .
cargo run -p kowalski-cli -- agent-app run --path . "your source text or URL"
```

Artifacts default to **`output/`** (see `horde.md`).

Born with **Rookery** (Kowalski 1.3.0).
