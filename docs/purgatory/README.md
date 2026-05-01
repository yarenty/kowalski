# Documentation archive (`purgatory`)

Files here are **kept for history** (blog-style articles, old static site snapshots, superseded diagrams). They are **not** maintained to match the current workspace layout or release line.

**Current canonical docs:** see [`../README.md`](../README.md) (index for `docs/`).

| File | Why it lives here |
|------|-------------------|
| `article_vesion050.md` | Milestone article for **v0.5.0** and the old multi-crate agent layout. |
| `article_data1.md` | Standalone “Data Agent” narrative; tools/data paths are now **`TemplateAgent` + tools** in `kowalski-core`. |
| `use_cases.md` | Enterprise pitch referencing **`kowalski-tools` / per-agent crates** topology; superseded by consolidated workspace docs. |
| `index.html`, `roadmap.html`, `modules.html`, `architecture.html` | Legacy static HTML site; duplicated root README/ROADMAP and used broken/missing image paths. Prefer repo markdown + [`../README.md`](../README.md). |
| `architeture.mermaid` | Old diagram (separate Web/Academic/Code/Data agent boxes). Filename typo; current architecture is in root README and [`OVERVIEW_1_1.md`](../OVERVIEW_1_1.md). |

To revive a document: copy it out of `purgatory/`, update naming and facts, then link it from [`../README.md`](../README.md).
