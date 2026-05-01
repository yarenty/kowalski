# Knowledge Compiler example

**Example aligned with workspace release line 1.1.0**

This example is a markdown-native **knowledge compiler**: ingest heterogeneous inputs → compile an Obsidian-style wiki → answer a focused question → lint/consistency report.  
It integrates three surfaces:

| Surface | Purpose |
|---------|---------|
| **`horde.md`** | Horde catalog for `kowalski` HTTP serve + Operator UI (**Horde Run** / Federation) |
| **`main-agent.md` + `agents/*.md`** | App spec parsed by **`kowalski-cli agent-app`** (list / validate / run / worker / delegate / proof) |
| **`prompts/`**, **`templates/`** | Prompt bodies and shaping templates referenced by specialists |

---

## Prerequisites

- **Rust** toolchain (`cargo`).
- **`kowalski`** HTTP server running locally (default `http://127.0.0.1:3456`) when using chat-backed steps or federation.
- **`[llm]` / Ollama** configured in repo root **`config.toml`** so `POST /api/chat` succeeds (same as the Operator UI Chat tab).
- Optional: **`bun` / `npm`** to run **`ui/`** for Horde Run.

---

## Where outputs go (important)

There are **two** artifact layouts, depending on how you run the workflow:

### 1) Horde path (UI **`Horde Run`**, or federation workers with horde orchestration)

The server reads **`horde.md`** and resolves a single **`workdir`**. Everything under **`workdir`** is the runtime tree (`raw`, `wiki`, `derived`, `scratch` as subfolders).

Typical layout after runs:

```text
examples/knowledge-compiler/output/        # horde.workdir (see horde.md; gitignored)
├── raw/sources/                           # ingest
├── wiki/                                  # Obsidian-oriented notes (delivery_root_rel = wiki)
├── derived/reports/                       # ask output, follow-ups, …
├── derived/lint/
└── scratch/workers/                       # worker logs when managed by serve
```

**Configure `workdir`** in **`horde.md`**:

- Prefer a **repository-relative** path so clones work everywhere, e.g. `workdir = "output"` (resolved relative to the horde manifest directory).
- The checked-in repo may use an absolute path for the maintainer machine; replace it locally if needed.

**Clean on startup** is controlled globally and per horde:

- **`config.toml`** — `[horde] clean_on_startup = true|false` applies when the horde does **not** override.
- **`horde.md`** — `config_on_startup = true|false` **or** alias `clean_on_startup` (same key as global; horde overrides global when set).
- The Operator UI displays the **effective** value (`GET /api/hordes*` includes `config_on_startup_effective`).

### 2) Standalone **`agent-app run`** path (CLI only, sequential pipeline)

Command **`cargo run -p kowalski-cli -- agent-app run ...`** executes the pipeline **in-process** and writes beside the manifests under the **app root** (default `examples/knowledge-compiler/`), **not** under `horde.workdir`:

```text
examples/knowledge-compiler/
├── raw/sources/
├── wiki/
├── derived/reports/
├── derived/lint/
└── scratch/                               # orchestration logs (orchestration-*.md)
```

These paths are **gitignored** at repo root (see `.gitignore`) so generated trees do not clutter commits.

---

## Source tree (committed)

Static definition only (no bundled shell `scripts/` or `config/*.yaml`; those were retired):

```text
examples/knowledge-compiler/
├── horde.md                 # Horde id, pipeline, workdir, delivery metadata, federation topic
├── main-agent.md            # agent-app pipeline + declared sub-agent names
├── agents/
│   ├── ingest.md
│   ├── compile.md
│   ├── ask.md
│   └── lint.md
├── prompts/
│   ├── compiler.md
│   ├── query.md
│   ├── lint.md
│   └── output.md
├── templates/
│   ├── concept.md
│   ├── source_summary.md
│   └── index.md
├── README.md
└── output/                  # horde workdir (created at runtime when using horde path; ignored)
```

**Capabilities** (prefix `kc` in this example):

| Step | Capability | Role |
|------|-------------|------|
| ingest | `kc.ingest` | Normalize inputs → `raw/sources/` |
| compile | `kc.compile` | Wiki + summaries under `wiki/` |
| ask | `kc.ask` | Answer → `derived/reports/` |
| lint | `kc.lint` | Report → `derived/lint/` |

Default worker **`default_agent_id`** values in **`agents/*.md`**: `kc-ingest`, `kc-compile`, `kc-ask`, `kc-lint`.

---

## Quick start — Operator UI (recommended)

From repo root:

1. Start API:

```bash
cargo run -p kowalski
```

2. Start **`ui`** (proxies `/api` to `127.0.0.1:3456`; see **`ui/vite.config.ts`**):

```bash
cd ui && bun install && bun run dev
```

3. Open the app → **Horde** → pick **Knowledge Sucking Swarm** (id `knowledge-compiler`).  

4. Obsidian consumption: sync or open **`workdir/wiki`** (shown in UI as Obsidian-ready path).  
   Use **Open output folder** — it invokes **`POST /api/system/open-path`** so the desktop file manager opens the path (avoid `file://` in the browser).

---

## Discovery of this horde

`kowalski` discovers **`horde.md`** under:

- Paths derived from **`KOWALSKI_HORDES_DIR`** (`:` separated), `<config-dir>/hordes`, **`examples`** next to config, cwd **`examples`**, and the built-in **`/opt/ml/kowalski/examples`** fallback.

Run serve from repo root **or** set **`KOWALSKI_HORDES_DIR`** to this example’s **`examples`** parent if needed.

---

## CLI — **`kowalski-cli agent-app`** (native)

Default app root: **`examples/knowledge-compiler`** (override with **`--path <dir>`** on list/validate/run/worker/proof).

```bash
# Help
cargo run -p kowalski-cli -- agent-app --help

# Inspect pipeline
cargo run -p kowalski-cli -- agent-app list

# Validate main-agent + agents/*.md consistency
cargo run -p kowalski-cli -- agent-app validate

# Sequential run (writes under app root dirs: raw/, wiki/, derived/, scratch/)
# Requires serve + working LLM for compile/ask/lint HTTP steps
cargo run -p kowalski-cli -- agent-app run "https://example.com/article" --question "What changed?"

# Custom app root / API URL
cargo run -p kowalski-cli -- agent-app run "https://example.com" --question "Summarize?" --path /abs/path/to/this-folder --api http://127.0.0.1:3456
```

### Federation helpers (workers + delegate)

Orchestration for **`kc.run`** and per-step **`kc.<step>`** is documented in **`kowalski-cli`**; typical dev flow:

```bash
# Terminal 1: server
cargo run -p kowalski

# Terminal 2: one worker — multi-step horde model uses ONE worker per role, e.g.:
cargo run -p kowalski-cli -- agent-app worker kc-ingest --path examples/knowledge-compiler --role ingest --api http://127.0.0.1:3456

# Repeat compile / ask / lint with distinct agent IDs, or drive everything from Operator UI Federation Management ("Start All").

# Raw delegate smoke (argument order: CAPABILITY SOURCE)
cargo run -p kowalski-cli -- agent-app delegate kc.ingest "https://example.com/article" --api http://127.0.0.1:3456

# Legacy single-capability bundled run instruction (printed by proof checklist)
cargo run -p kowalski-cli -- agent-app delegate kc.run "https://example.com/article" --question "What changed?" --api http://127.0.0.1:3456
```

**`agent-app proof`** prints a repeatable checklist (oriented toward the legacy **`kc.run` + one worker** story); for the **four-worker horde**, prefer UI + federation event stream):

```bash
cargo run -p kowalski-cli -- agent-app proof --path examples/knowledge-compiler --api http://127.0.0.1:3456
```

---

## Extensions (optional wrapper)

Discovery: **`cargo run -p kowalski-cli -- extension list`** (PATH **`kowalski-ext-<name>`** or **`.kowalski/extensions/<name>/run`**).

If your environment still ships **`knowledge-compiler`** wrapper:

```bash
cargo run -p kowalski-cli -- extension run knowledge-compiler help
```

Prefer **`agent-app`** for accurate flags; avoid outdated **`config/`**, **`scripts/`**, or **`extension run`** examples that referenced removed paths.

---

## HTTP touchpoints

| Endpoint | Role |
|---------|------|
| `POST /api/hordes/{id}/run` | Horde orchestrated run |
| `GET /api/federation/stream` | SSE federation / run events |
| `POST /api/system/open-path` | Open **`workdir`** in OS file manager from UI |
| `POST /api/chat` | **`agent-app`** compile/ask/lint when not using inlined ingest fetch only |

---

## Troubleshooting

- **UI says LLM/API errors**: confirm **`cargo run -p kowalski`** and Ollama (or **`[llm]`** provider) per root **`config.toml`**.
- **Horde workers not READY**: use Federation Management **Start All** or start four **`agent-app worker … --role <step>`** processes with IDs matching **`default_agent_id`** in **`agents/*.md`**.
- **Wrong artifact location**: Horde UI runs use **`horde.md` → `workdir`**; **`agent-app run`** uses files next to **`main-agent.md`** (see sections above).

---

## What changed vs older README scaffolds

- Removed **`config/`**, **`scripts/`**, stale **`agents.yaml`** / **`pipeline.yaml`** references.
- Documented **`horde.md`**, **`workdir`** / **`output/`**, **`[horde].clean_on_startup`**, UI **Open folder** backend.
- Canonical CLI prefix is **`kowalski-cli agent-app`**, not only **`extension run knowledge-compiler`**.
