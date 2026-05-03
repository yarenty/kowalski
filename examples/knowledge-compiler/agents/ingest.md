---
name = "ingest"
kind = "ingest"
capability = "kc.ingest"
default_agent_id = "kc-ingest"
display_name = "Ingest Agent"
description = "Collects raw source material and stores normalized markdown."
output = "raw/sources/"
---

# Ingest Agent

Collects raw source material and stores normalized markdown in `raw/sources/`.

When delegated `kc.ingest`, the worker fetches the URL (or captures input text), writes a timestamped source file, and returns its absolute path so the next stage can read it.

## URL capture (worker)

Bundling uses `kowalski_core::source_bundle` (no horde-specific fetch rules in the CLI):

- **github.com** URLs that resolve as GitHub repos or raw paths → GitHub ingest (README API / raw / plain HTTP as appropriate). If that path fails, the worker falls back once to the generic web fetch.
- **Other HTTP(S)** URLs → generic GET; when the body looks like HTML, a small strip converts it to markdown-ish text.

Set **`GITHUB_TOKEN`** in the worker environment for private repos or better rate limits.

## Vault / Obsidian

There is **no** Obsidian import API in this repo. Output is **markdown on disk** under the horde `workdir/` (for example `wiki/`). Bringing notes into a vault is an **operator** step: copy, `rsync`, sync folder, or Git—whatever fits your vault layout.
