# URL Summarizer Horde

Reads the pages at the URLs you give it and writes a short Markdown summary of each into one hand-off file.

## Quick start

```bash
cargo run -p kowalski-cli -- agent-app validate --path .
cargo run -p kowalski-cli -- agent-app run --path . "your source text or URL"
```

Artifacts default to **`output/`** (see `horde.md`).

Born with **Rookery** (Kowalski 1.3.0).
