---
name = "ingest"
kind = "ingest"
capability = "kc.ingest"
default_agent_id = "kc-ingest"
display_name = "Ingest Agent"
description = "Collects raw source material and stores normalized markdown."
output = "debug/raw/"
[[inputs]]
id = "question"
type = "text"
label = "What should the note answer?"
required = true
placeholder = "What are the main ways to learn Rust?"
[[inputs]]
id = "sources"
type = "textarea"
label = "Pages or notes to read, one per line"
required = true
placeholder = "https://www.rust-lang.org/learn\nhttps://github.com/rust-lang/rustlings"
---

# Ingest Agent

Collects raw source material and stores normalized markdown in `debug/raw/` (timestamped bundle `.md` files).

When delegated `kc.ingest`, the worker fetches the URL (or captures input text), writes a timestamped source file, and returns its absolute path so the next stage can read it.

## URL capture (worker)

Bundling uses `kowalski_core::source_bundle` (no horde-specific fetch rules in the CLI):

- **github.com** URLs that resolve as GitHub repos or raw paths → GitHub ingest (README API / raw / plain HTTP as appropriate). If that path fails, the worker falls back once to the generic web fetch.
- **Other HTTP(S)** URLs → generic GET; when the body looks like HTML, a small strip converts it to markdown-ish text.

Set **`GITHUB_TOKEN`** in the worker environment for private repos or better rate limits.

## Vault / Obsidian

With a notes vault set (Setup → Notes vault, or `[vault] dir`), the finished note is saved into the vault's **`Kowalski`** folder by itself. Without one, copy **`workdir/PASTE_ME.md`** into your vault. Raw captures live under **`workdir/debug/raw/`** for debugging only.
