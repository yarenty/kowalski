# Knowledge Compiler example — agent / operator guide

> **Read this** before changing prompts, the horde manifest, or **`kowalski-core` internal tool** behavior used by ingest for this app.

## Principles (this example)

These apply to **this** Knowledge Compiler horde / `agent-app` example only. **Other hordes** in Kowalski may use different workdir layouts, primary artifacts, and processing—there is no global requirement to match this pattern.

- **Straightforward defaults (here):** one operator-facing file at the workdir root (**`PASTE_ME.md`**), produced by this app’s **`lint`** + `write_paste_me_file` path. No extra flags for that behavior in this example.
- **`debug/` (here):** this example puts intermediates under **`workdir/debug/`** for monitoring and tooling. That directory name and role are **not** a framework-wide contract; another horde might write only JSON, binary artifacts, or a flat tree with different names.

## What this is

A **markdown-native knowledge pipeline**: **ingest** → **compile** → **ask** → **lint**.  
**Single manifest:** **[`horde.md`](horde.md)** defines the horde **id**, **pipeline** order, **`workdir`**, delivery copy, federation topic, and (with the server) loads **`agents/*.md`** for each step. **`kowalski-cli agent-app`** reads the **same** `horde.md` + `agents/` for **`list`**, **`validate`**, **`run`**, and **`worker --role`**.

For **this** app, delivery is **files** under `workdir`. For quick repeated runs, use **`PASTE_ME.md`** at the workdir root (and the Horde UI **Copy to clipboard** on `run_finished` when wired). Other pipeline output lives under **`debug/`** for monitoring only.

## Surfaces

| Surface | Entry | Notes |
|--------|--------|--------|
| Horde + UI | [`horde.md`](horde.md) | `workdir` holds runtime artifacts; **layout is defined by this app** (see below), not by Horde generically. |
| CLI | `cargo run -p kowalski-cli -- agent-app …` | Same **`horde.md`** + **`agents/`**; default **`output/`** workdir matches **`horde.md`**. |
| Worker | `agent-app worker --role <ingest\|compile\|ask\|lint>` | One role per process for federation (`kind` in `agents/<step>.md`). |

## Workdir layout (this example)

- **`PASTE_ME.md`** — for this horde, the main hand-off at the workdir root; copy into Obsidian (regenerated after `lint` in this pipeline).
- **`debug/`** — in this example, all intermediate / monitoring output:
  - `debug/raw/sources/` — ingest
  - `debug/wiki/` — concepts, summaries, `index.md` (extra top-level folders under `wiki/` still appear in **Bundled reference** in `index.md` if you add them manually)
  - `debug/derived/reports|lint/` — ask, lint
  - `debug/scratch/` — orchestration logs

## GitHub URL capture (internal tool, MCP optional)

Horde **ingest** uses **`kowalski_core::source_bundle`** (GitHub-aware fetch + HTML→Markdown when the response looks like HTML). It resolves common GitHub URLs to useful text:

- `https://github.com/owner/repo` → [README API](https://docs.github.com/en/rest/repos/repos#get-a-repository-readme) (`Accept: application/vnd.github.raw+json`).
- `https://github.com/owner/repo/blob/ref/path/to/file.md` → `raw.githubusercontent.com` raw file.

Set **`GITHUB_TOKEN`** for higher rate limits and private repos (`Authorization: Bearer` on API/raw requests).

If GitHub-specific fetching fails, ingest **falls back** to a plain HTTP GET of the original URL (often HTML).

## MCP vs internal tools

| Use case | Recommendation |
|----------|----------------|
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

## Worker roles

| Role | Capability |
|------|------------|
| ingest | `kc.ingest` |
| compile | `kc.compile` |
| ask | `kc.ask` |
| lint | `kc.lint` |

## Troubleshooting

- **Wrong artifact path**: For this example, Horde and standalone **`agent-app run`** both use **`output/`** under the app root (`horde.workdir` and the CLI default match).
- **Validate errors**: every **`horde.md`** pipeline step must have **`agents/<step>.md`**; do not leave extra agent files that are not listed in the pipeline.
- **GitHub HTML instead of README**: use repo URL `https://github.com/o/r` or blob URL to the file; set `GITHUB_TOKEN` if rate-limited.

## Related docs

- [`README.md`](README.md) — quick start, prerequisites.
- Root [`AGENTS.md`](../../AGENTS.md) — repository-wide rules.
