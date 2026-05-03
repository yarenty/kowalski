# Knowledge Compiler example

**Example aligned with workspace release line 1.1.0**

Operator-focused behavior, GitHub ingest, and federation roles are documented in **[`AGENTS.md`](AGENTS.md)**.

This example is a markdown-native **knowledge compiler**: ingest heterogeneous inputs → compile an Obsidian-style wiki → answer a focused question → lint/consistency report.  
It integrates three surfaces:

| Surface | Purpose |
|---------|---------|
| **`horde.md` + `agents/*.md`** | Single spec: **`kowalski`** (Horde UI / federation) and **`kowalski-cli agent-app`** (list / validate / run / worker / delegate / proof) |
| **`prompts/`**, **`templates/`** | Prompt bodies and shaping templates referenced by specialists |

---

## Prerequisites

- **Rust** toolchain (`cargo`).
- **`kowalski`** HTTP server running locally (default `http://127.0.0.1:3456`) when using chat-backed steps or federation.
- **`[llm]` / Ollama** configured in repo root **`config.toml`** so `POST /api/chat` succeeds (same as the Operator UI Chat tab).
- Optional: **`bun` / `npm`** to run **`ui/`** for Horde Run.

---

## Where outputs go (important)

**Horde UI**, **federation workers**, and **`agent-app run`** all use the same **`workdir`** from **`horde.md`** (default **`output/`** under this example). Operators care about **`PASTE_ME.md`** at the workdir root; **everything else** is under **`debug/`** (ingest, wiki, reports, scratch — for monitoring only).

### Layout (Horde UI or `agent-app run`)

Typical layout after runs:

```text
examples/knowledge-compiler/output/        # horde.workdir (see horde.md; gitignored)
├── PASTE_ME.md                            # copy into Obsidian (delivery_root_rel default)
├── debug/
│   ├── raw/sources/                       # ingest
│   ├── wiki/                              # compiled notes + index
│   ├── derived/reports/                   # ask, follow-ups, …
│   ├── derived/lint/
│   └── scratch/                           # orchestration logs
└── scratch/workers/                       # only when managed by serve (optional)
```

**Configure `workdir`** in **`horde.md`**:

- Prefer a **repository-relative** path so clones work everywhere, e.g. `workdir = "output"` (resolved relative to the horde manifest directory).
- The checked-in repo may use an absolute path for the maintainer machine; replace it locally if needed.

**Clean on startup** is controlled globally and per horde:

- **`config.toml`** — `[horde] clean_on_startup = true|false` applies when the horde does **not** override.
- **`horde.md`** — `config_on_startup = true|false` **or** alias `clean_on_startup` (same key as global; horde overrides global when set).
- The Operator UI displays the **effective** value (`GET /api/hordes*` includes `config_on_startup_effective`).

**`agent-app run`** writes the same tree under **`output/`** (see layout above). The **`output/`** tree is **gitignored** (see `.gitignore`).

---

## Source tree (committed)

Static definition only (no bundled shell `scripts/` or `config/*.yaml`; those were retired):

```text
examples/knowledge-compiler/
├── AGENTS.md                # Operator guide (GITHUB_TOKEN, MCP vs CLI)
├── horde.md                 # Horde id, pipeline, workdir, delivery, federation (also drives agent-app)
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
| ingest | `kc.ingest` | Normalize inputs → `debug/raw/sources/` |
| compile | `kc.compile` | Wiki + summaries under `debug/wiki/` |
| ask | `kc.ask` | Answer → `debug/derived/reports/` |
| lint | `kc.lint` | Report → `debug/derived/lint/` (+ writes `PASTE_ME.md`) |

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

4. Obsidian consumption: sync or open **`workdir/debug/wiki/`** (shown in UI as Obsidian-ready path).  
   Use **Open output folder** — it invokes **`POST /api/system/open-path`** so the desktop file manager opens the path (avoid `file://` in the browser).

---

## Discovery of this horde

`kowalski` discovers **`horde.md`** under:

- Paths derived from **`KOWALSKI_HORDES_DIR`** (`:` separated), `<config-dir>/hordes`, **`examples`** next to config, cwd **`examples`**, and the built-in **`/opt/ml/kowalski/examples`** fallback.

Run serve from repo root **or** set **`KOWALSKI_HORDES_DIR`** to this example’s **`examples`** parent if needed.

---

## CLI — **`kowalski-cli agent-app`** (native)

Default app root: env **`KOWALSKI_AGENT_APP_ROOT`**, else **`examples/knowledge-compiler`** (override with **`--path <dir>`** on list/validate/run/worker/proof).

```bash
# Help
cargo run -p kowalski-cli -- agent-app --help

# Inspect pipeline
cargo run -p kowalski-cli -- agent-app list

# Validate horde.md pipeline vs agents/*.md
cargo run -p kowalski-cli -- agent-app validate

# Sequential run (writes under output/ — same workdir as horde.md)
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
- **Wrong artifact location**: Both Horde UI and local **`agent-app run`** use the same default **`workdir`** for this example: **`output/`** under the app root (see **`horde.md`** and the layout section above).

---

## What changed vs older README scaffolds

- Removed **`config/`**, **`scripts/`**, stale **`agents.yaml`** / **`pipeline.yaml`** references.
- Documented **`horde.md`**, **`workdir`** / **`output/`**, **`[horde].clean_on_startup`**, UI **Open folder** backend.
- Canonical CLI prefix is **`kowalski-cli agent-app`**, not only **`extension run knowledge-compiler`**.
