# `docs/` — documentation index

Use this folder for **design articles**, **architecture notes**, and **long-form explanations**. Operational entry points remain the repository **[`README.md`](../README.md)** and **[`CHANGELOG.md`](../CHANGELOG.md)**.

## Start here (1.1.x)

| Doc | Purpose |
|-----|---------|
| [`OVERVIEW_1_1.md`](./OVERVIEW_1_1.md) | Short narrative of the **1.1.0** line: horde workflows, Knowledge Compiler, extensions, federation UX. |
| [`DESIGN_MEMORY_AND_DEPENDENCIES.md`](./DESIGN_MEMORY_AND_DEPENDENCIES.md) | Canonical memory stack rationale (dependency-light defaults, Qdrant as PoC). |
| [`../examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md) | First **horde-style app** example (ingest → compile → ask → lint). |

## Memory

| Doc | Purpose |
|-----|---------|
| [`memory_architecture.md`](./memory_architecture.md) | Three-tier memory model (working / episodic / semantic). |
| [`article_memory.md`](./article_memory.md) | Longer-form article on agent memory (aligned with `kowalski-core` memory modules). |

## Tools & technology

| Doc | Purpose |
|-----|---------|
| [`article_tooling.md`](./article_tooling.md) | Principles for designing tools for **`TemplateAgent`** and the tool chain. |
| [`key_technology.md`](./key_technology.md) | Perspectives (technology, business, research) — updated for the **1.1.x** workspace layout. |

## Archive

| Location | Purpose |
|----------|---------|
| [`purgatory/`](./purgatory/README.md) | Old articles and static HTML **not** kept in sync with mainline releases. |

## Images & assets

Illustrations and exports live under [`img/`](./img/). Prefer referencing diagrams from markdown in this tree rather than duplicating a second HTML site.

## Link checking (CI & local)

GitHub Actions runs **[Lychee](https://github.com/lycheeverse/lychee)** on all `**/*.md` files (`offline` mode: validates repo-relative paths and existing files; skips external URLs). Config: [`.lychee.toml`](../.lychee.toml).

```bash
cargo install lychee   # once
just docs-links        # or: ./scripts/docs-linkcheck.sh
```
