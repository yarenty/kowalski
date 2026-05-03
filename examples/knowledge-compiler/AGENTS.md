# Knowledge Compiler example — agent / operator guide

> **Read this** before changing prompts, the horde manifest, or **`kowalski-core` internal tool** behavior used by ingest for this app.

## What this is

A **markdown-native knowledge pipeline**: **ingest** (collect sources) → **compile** (LLM + wiki) → **ask** (Q&A) → **lint** (consistency). Optional: **research** (seed/tip → investigation packet).  
It is **not** a full Obsidian or mdBook product: delivery is **files** under a `workdir` with `[[wikilinks]]` and optional **merging** into an existing [mdBook](https://rust-lang.github.io/mdBook/) repo (e.g. [dev_tips](https://github.com/yarenty/dev_tips)).

## Surfaces

| Surface | Entry | Notes |
|--------|--------|--------|
| Horde + UI | [`horde.md`](horde.md) | `workdir` holds all runtime artifacts. |
| CLI | `cargo run -p kowalski-cli -- agent-app …` | Writes under app root if not using horde workdir. |
| Worker | `agent-app worker --role <ingest\|compile\|ask\|lint\|research>` | One role per process for federation. |

## Workdir layout

- `raw/sources/` — combined ingest output (markdown).
- `wiki/concepts/`, `wiki/summaries/` — compiled notes; `wiki/index.md` is auto-generated.
- `derived/reports/`, `derived/lint/`, `derived/research/` — ask, lint, optional research.
- `derived/mdbook-summary-suggestion.md` — when `external_vault_root` is set, **suggested** `SUMMARY.md` lines (apply manually).
- `scratch/` — orchestration logs.

## Optional mdBook / dev_tips merge

In [`main-agent.md`](main-agent.md) TOML frontmatter you can add (all optional):

- `external_vault_root` — path to a clone of your book repo (absolute or **relative to the app root**), e.g. `../dev_tips`.
- `mdbook_doc_rel` — subdirectory for markdown (default `doc`).
- `corpus_budget_chars` — max characters of existing `**/*.md` under that doc tree to inject into **compile** and **research** (default `120000`).

**Compile** will prepend a “Existing vault corpus” section so new notes can link to real titles. **No** automatic edit of your clone’s `SUMMARY.md` — use `derived/mdbook-summary-suggestion.md` and merge by hand until you trust automation.

**Clone workflow (typical):**

```bash
git clone https://github.com/yarenty/dev_tips.git
# In main-agent.md set: external_vault_root = "../dev_tips"  (or your path)
```

## GitHub URL capture (internal tool, MCP optional)

Horde **ingest** uses **`kowalski_core::source_bundle`** (GitHub-aware fetch + HTML→Markdown when the response looks like HTML). It resolves common GitHub URLs to useful text:

- `https://github.com/owner/repo` → [README API](https://docs.github.com/en/rest/repos/repos#get-a-repository-readme) (`Accept: application/vnd.github.raw+json`).
- `https://github.com/owner/repo/blob/ref/path/to/file.md` → `raw.githubusercontent.com` raw file.

Set **`GITHUB_TOKEN`** for higher rate limits and private repos (`Authorization: Bearer` on API/raw requests).

If GitHub-specific fetching fails, ingest **falls back** to a plain HTTP GET of the original URL (often HTML).

## MCP vs internal tools

| Use case | Recommendation |
|----------|------------------|
| Horde/automation | Internal GitHub + HTTP helpers in core today; workers stay thin. |
| Rich GitHub / OAuth / search | Use an **MCP** server (in-repo, stdio, or [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/) profile); wire through `McpHub` when horde steps invoke tools. |
| Interactive exploration | Configure MCP on your **chat** agent in `config.toml`; paste URLs or files into ingest. |

Canonical layering: [`kowalski-core/AGENTS.md`](../../kowalski-core/AGENTS.md) — **Tool execution model (three sources)**.

## Optional Docker: HTML → cleaner text

For arbitrary URLs that return HTML, you can run a **readability/trafilatura** sidecar and paste/save its markdown output as a **local file path** ingest input (supported today). Example outline:

```yaml
# compose snippet — adapt image/command to your preference
services:
  readability:
    image: localhost/readability-service   # hypothetical
    ports: ["8080:8080"]
```

Then: `POST` URL → receive markdown → save to `/tmp/page.md` → ingest `cargo run -p kowalski-cli -- agent-app run /tmp/page.md …`

Keep this **optional**; the repo does not ship a compose file by default.

## Relationships (`[[wikilinks]]`)

- **Compile** prompt asks for concept links; [`normalize_and_repair_wiki`](../../kowalski-cli/src/agent_app_ops.rs) ensures stubs for links from **summaries** (Sources backlink) and from **concept pages** (reciprocal **Related Concepts** backlinks).
- Templates include **`extends`** / **`see_also`** hints — use real existing note titles when linking (e.g. Byobu builds on `[[tmux]]`).

## Research seed (“tip line” → structured packet)

1. **Ingest** a URL or paste text (short tip).
2. Either run **`research`** as a pipeline step after ingest (see below), or run manually:

```bash
cargo run -p kowalski-cli -- agent-app run \
  "mcp-remote: …" --path examples/knowledge-compiler
# Custom pipeline: ingest -> research (edit main-agent.md pipeline)
```

3. Output: `derived/research/latest.md` — promotion into `dev_tips/doc/…` is manual or scripted outside Kowalski.

Default [`main-agent.md`](main-agent.md) keeps the original four-step pipeline; add **`research`** to `pipeline` only when you want this step in UI/CLI runs.

## Worker roles

| Role | Capability |
|------|------------|
| ingest | `kc.ingest` |
| compile | `kc.compile` |
| ask | `kc.ask` |
| lint | `kc.lint` |
| research | `kc.research` |

## Troubleshooting

- **Wrong artifact path**: Horde runs use `horde.workdir`; standalone `agent-app run` uses paths beside `main-agent.md`.
- **GitHub HTML instead of README**: use repo URL `https://github.com/o/r` or blob URL to the file; set `GITHUB_TOKEN` if rate-limited.
- **Vault corpus missing**: check `external_vault_root` resolves from app root; confirm `mdbook_doc_rel` exists.

## Related docs

- [`README.md`](README.md) — quick start, prerequisites.
- Root [`AGENTS.md`](../../AGENTS.md) — repository-wide rules.
