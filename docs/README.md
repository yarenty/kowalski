# `docs/`

Start with **[architecture.html](architecture.html)**: how kowalski is built, with diagrams of the
system, one run, and the run lifecycle. Operational entry points stay in the repository
[README](../README.md) and [CHANGELOG](../CHANGELOG.md).

| Folder | What lives there | Kept in sync with the code? |
|---|---|---|
| [`dev/`](dev/) | Technical notes for people changing kowalski | Yes: a change that makes one wrong updates it |
| [`blog/`](blog/) | Articles written to be published | Dated; facts correct when written |
| [`concepts/`](concepts/) | Ideas and designs not built (or not yet) | No |
| [`purgatory/`](purgatory/README.md) | Superseded documents kept for history | No |
| [`img/`](img/) | Diagrams and pictures used by the above | — |

## dev/

| Doc | Purpose |
|---|---|
| [`DESIGN_MEMORY_AND_DEPENDENCIES.md`](dev/DESIGN_MEMORY_AND_DEPENDENCIES.md) | Why the memory stack is dependency-light, and when to reach for Postgres |
| [`memory_architecture.md`](dev/memory_architecture.md) | The three memory tiers: working, episodic, semantic |
| [`WORKFLOW_MANIFEST.md`](dev/WORKFLOW_MANIFEST.md) | The portable workflow manifest: JSON form of a horde, schema, converters, round trip |
| [`GOVERNANCE.md`](dev/GOVERNANCE.md) | Who owns which docs and when they must change |
| [`demo/`](dev/demo/) | Terminal recording script for the Spreadsheet analyst demo |

## blog/

| Article | About |
|---|---|
| [`architecture-history.md`](blog/architecture-history.md) | Eighteen months of kowalski: how the architecture kept getting smaller |
| [`article_memory.md`](blog/article_memory.md) | Building human-like memory for agents |
| [`article_tooling.md`](blog/article_tooling.md) | Your agent is only as good as its tools |

Published copies live on the kowalski blog.

## concepts/

| Doc | Idea |
|---|---|
| [`architecture_v03_future.md`](concepts/architecture_v03_future.md) | An earlier future-state architecture |
| [`DESIGN_A2A_FEDERATION_EDGE.md`](concepts/DESIGN_A2A_FEDERATION_EDGE.md) | Agent-to-agent protocol at the federation edge |
| [`key_technology.md`](concepts/key_technology.md) | Technology, business and research perspectives |

## Link checking

CI runs [Lychee](https://github.com/lycheeverse/lychee) offline on every `*.md` (repo-relative
links must resolve); `purgatory/` is excluded. Config: [`.lychee.toml`](../.lychee.toml).
Locally: `cargo install lychee-cli`, then `./scripts/docs-linkcheck.sh` or `just docs-links`.
