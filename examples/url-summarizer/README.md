# URL Summarizer Horde

Ingests URLs from the operator, summarizes their content into Markdown, and compiles a final handoff file.

## Quick start

```bash
cargo run -p kowalski-cli -- agent-app validate --path .
cargo run -p kowalski-cli -- agent-app run --path . "your source text or URL"
```

Artifacts default to **`output/`** (see `horde.md`).

Born with **Rookery** (Kowalski 1.3.0).
