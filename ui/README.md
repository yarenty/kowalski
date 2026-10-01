# Kowalski UI (Vue 3 + Vite)

**Version 2.6.0** · Operator-facing web shell for Kowalski, calling **`kowalski`** under `/api/*`.

Features: health, MCP ping, **Chat** (`POST /api/chat`, SSE **`POST /api/chat/stream`** with optional **Tool-aware stream** / `tools_stream`), federation, graph extension status. See [`ROADMAP.md`](./ROADMAP.md).

## Look and navigation (Commando theme)

- **Top bar**: the penguin mark and KOWALSKI wordmark, then **Hordes** (the landing screen once
  Setup is done), **Runs**, **Chat**, **Build** (the Rookery horde builder) and **Setup**, and an
  **Admin ▾** menu with **Federation**, **MCP servers**, **Graph**, **Diagnostics** (API status
  and the API token) and **About**. On the right: **Find a horde or a run… ⌘K** (opens the picker)
  and a compact light / dark / auto toggle. Below ~1000px the nav folds into one menu. A red count
  on **Runs** says how many runs wait for approval. Internal tab ids are unchanged.
- **Hordes home**: "What do you need done?" with filter chips (Pinned · Spreadsheets · Web &
  news · Documents · Code when present · All N) over a grid of horde tiles: a coloured square with
  the horde's icon, its name, the first sentence of its description and one quiet status line
  (next scheduled time, the folder it watches, or the last run). **Pinned** shows the operator's
  starred hordes (kept in this browser under `kowalski.ui.hordes.pinned.v1`), or the horde's
  shipped `featured` flag until something is starred. **All** uses smaller tiles and shows at most
  12 before **Show all**, so 100 hordes stay calm. A red strip at the top appears only when a run
  waits for approval (**Review** opens it).
- **Horde page** (from a tile, the picker, the Runs page or a deep link): breadcrumb, icon, name
  and description; the request form with **Send in the horde**; on the right the last five runs
  of this horde and **All runs →**, with "How this horde works", schedules and the output folder
  folded away. While a run works, the title becomes the run's title, then the pipeline stepper
  (done ✓ / current red / pending / failed ✕) and one **Now** card (current step, elapsed time,
  what it does); the activity log is folded. A finished run shows the answers first: the
  **Delivered** card with the rendered hand-off and the output file, then folded rows for
  intermediate files, how it was computed, the Markdown source, the activity log and the raw
  payload, then **Follow-up**. A run waiting before a command step shows the red approval box at
  the top (Approve / Cancel run); interrupted runs sit in a calm banner with **Resume**.
- **Runs page**: every run across hordes, grouped by day, with filter chips (All / Needs you /
  Failed, Running while any run works) and an **Any horde** select; 25 at a time with **Load
  older**. A row opens that run on its horde page.
- **⌘K picker** (Cmd+K / Ctrl+K anywhere, or the top-bar box): search hordes (name,
  description, category) and the last 50 runs (title); ↑↓ move, ↵ open, Esc close.
- **Icons**: [`src/hordeIcons.ts`](./src/hordeIcons.ts) owns the category list (mirroring
  `HORDE_CATEGORIES` in `kowalski/src/horde.rs`), each category's colour token and default icon,
  and the icon set (table, sunrise, inbox, link, book, code, blocks, file, globe, receipt, tag,
  chart, calendar, search, spark). A horde picks one with `icon = "…"` in `horde.md`.
- **Deep links**: `?tab=<id>` opens a tab (`federation-run`, `runs`, `chat`, `rookery`, `setup`,
  `federation-management`, `mcp`, `graph`, `home`, `about`), `?horde=<id>` opens that horde's
  page, `?horde=<id>&run=<run_id>` opens one of its runs, `?theme=light|dark|system` overrides the
  theme for that page load. The address bar follows navigation, so Back works.
- **Theme**: light / dark / auto toggle in the top bar (stored in `localStorage` under
  `kowalski.ui.theme.v1`; auto follows the OS). All colours are design tokens defined once in
  [`src/styles/theme.css`](./src/styles/theme.css) — components use only `var(--…)`, never hex
  literals. Text/background pairs are chosen for WCAG AA contrast.
- **Fonts**: Archivo (headings), IBM Plex Sans (body), JetBrains Mono (labels, ids, code), loaded
  from Google Fonts with system fallbacks, so the UI stays readable offline.

## Horde changes in 1.1.0 (since 1.0.0)

- Federation panel now supports clearer horde run observability with task progress events.
- Knowledge Compiler delegate/worker runs surface serialized step progress and final artifact delivery context in the UI flow.

## Setup

```bash
cd ui
bun install
bun run dev
```

Open [http://localhost:5173](http://localhost:5173)

Vite uses Rollup internally (transitive dependency). You do not need to add Rollup directly, but it still needs the matching native package for your runtime architecture.

If you hit `Cannot find module @rollup/rollup-darwin-*` on macOS, your runtime arch is usually mismatched (for example Rosetta x64 vs arm64). Use Node 22 arm64 and reinstall:

```bash
cd ui
rm -rf node_modules bun.lockb
bun install
```

## Build

```bash
bun run build
```

Static output is written to `dist/`. The next `cargo build -p kowalski` compiles it into the
server binary, which then serves the UI itself at `http://127.0.0.1:3456/` (no Node, no second
terminal). `bun run dev` with the Vite proxy remains the fast loop while editing the UI.

## Backend (HTTP API)

In one terminal from the repo root:

```bash
cargo run -p kowalski -- -c config.toml
```

This binds **`127.0.0.1:3456`** and serves JSON under `/api` (`/api/health`, `/api/doctor`, `/api/mcp/servers`, `POST /api/mcp/ping`, **`POST /api/chat`**, **`POST /api/chat/stream`** (body may include **`tools_stream`: true**), **`POST /api/chat/reset`**). With **`kowalski --features postgres`** and a Postgres memory URL, graph routes may include **`POST /api/graph/cypher`** (Apache AGE on the server). Use `-c` / `--ollama-url` as needed (see `kowalski --help`).

**API token (only when the server runs with auth enabled):** auth is **off by default** — no token needed. If the server was started with `--auth` (or `[server] auth = true` / `KOWALSKI_API_TOKEN` set), everything under `/api/*` except `/api/health` requires a bearer token generated at first server start (printed once; persisted at `<config-dir>/db/api_token`, path in the server log). In the UI, paste it into **Admin → Diagnostics → API token** (stored in this browser's `localStorage`) or answer the first-run prompt; for dev you can also set `VITE_API_TOKEN`.

## API proxy

`vite.config.ts` proxies `/api` to `http://127.0.0.1:3456` so the Vue app can call relative paths like `/api/health`. For a production build on another origin, set `VITE_API_BASE` to the full API origin (no trailing slash).

## Operator smoke checklist (~2 minutes)

Use this after any change to **`kowalski`**, **`kowalski-core`**, or **`ui/`** that could affect `/api/*`, horde catalog, federation, or chat. Full governance: [`AGENTS.md`](./AGENTS.md) (**UI-first**).

**Prerequisites**

1. Repo root: `cargo run -p kowalski -- -c config.toml` (default `http://127.0.0.1:3456`).
2. Second terminal: `cd ui && bun install && bun run dev` → open [http://localhost:5173](http://localhost:5173).

**Steps**

| # | Screen | What to do | Pass criteria |
|---|-------------|------------|-----------------|
| 1 | **Admin → Diagnostics** | Open once. Only if the server runs with `--auth`: paste its API token into **API token** → **Save** (first run only) | No blank crash; **Refresh all** shows agents/sessions (no 401 errors). |
| 2 | **Chat** | Send one short message | **Optional** if `[llm]` / Ollama is configured: you get a normal reply or a **clear** error in the thread (not a silent hang). Skip if you have no LLM. |
| 3 | **Admin → Federation** | Scroll to **Knowledge Compiler** → **Start All** | Workers move toward ready; no permanent red error. If workers never become ready, start matching `agent-app worker … --role …` processes from [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md). |
| 4 | **Hordes** | Pick a tile (or ⌘K). **Knowledge Compiler**: question + pages form → **Send in the horde**. **Rust Project Scaffolder** (`examples/rust-project-scaffolder`): operator form (project name, goals, crate shape) then **Send in the horde** | The pipeline stepper advances, the **Now** card names the working step and the activity log shows each step, or an explicit failure; the run appears under **Runs**. Scaffolder ingest needs valid `output` paths (auto-repaired on birth/repair). |
| 5 | **Build** (Rookery) | **New session** → describe a 3-step workflow → **Propose horde** → **Give birth** | Summary + pipeline on the right (horizontal track, or layered **DAG** canvas when `edges[]` is present); birth shows path under `examples/<id>/`. Run `cargo run -p kowalski-cli -- agent-app validate --path examples/<id>` to confirm. Requires live LLM for chat/propose. |
| 5b | **Hordes — Coder** | Restart server; select **Coder (planning tier)** → **Start All** → run | DAG canvas; project path + task form; `HANDOFF.md` under `examples/coder/output/`. |
| 5c | **Hordes — resume** | Kill the server mid-run; restart; open the horde again | **Interrupted runs** banner lists the run (status + resume attempts); **Resume** continues from the next ready step (completed steps keep artifacts) and the feed shows a "run resumed" marker. |
| 5d | **Hordes — cancel** | Start a run; click **Cancel run** in the **Now** card | The page says the run was cancelled; **Past runs** and **Runs** list it as Cancelled; remaining steps are skipped. No worker processes are involved — steps run in-process. |
| 5e | **Hordes — triggers** | Open a horde with `[[triggers]]` → **Schedules and watchers**: **Switch off / on** one, click **Run now**; restart the server and open it again | The switch flips on/off (marked "changed here") and **survives the restart**; Run now opens the started run; the home tile shows "Next: …" for an armed schedule or "Watching …" for an armed watcher. Trigger cards also badge on **Admin → Federation**. |
| 6 | **Admin → Federation** (optional extra) | Lower on the same panel: **Refresh registry** if you use raw delegate / `kc.run` smoke | Registry JSON loads; see [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md) for legacy worker commands. |



**Rust scaffolder demo** (Rookery → Horde):

1. **Rookery**: use the TEST PROMPT below → **Propose horde** → **Give birth** → `examples/rust-project-scaffolder/`
2. Restart `kowalski` (reload horde catalog) → **Horde** tab → select **Rust Project Scaffolder Pipeline**
3. Fill the **Operator input** form (project name, goals, optional URL, crate shape) → **Run horde**

If an older born horde still has `output = "String"`, call `POST /api/hordes/rust-project-scaffolder/repair-outputs` or re-birth from Rookery with overwrite.

TEST PROMPT (Rookery):
```txt
When having new rust project could you create pipeline to setup initial repository - project structure, invetigate crates that could be user for project, create first initial mock/mvp of the project and suggest todo list
```

**Failure triage**

- **CORS / network**: confirm Vite dev proxy and that the browser URL is the Vite origin (5173), not the API port directly.
- **Horde run stuck**: workers not started or wrong topic — return to step 3 and server logs.
- **Chat only**: horde can still be healthy; file issues separately if Chat breaks but Horde passes.

## See also

- [`AGENTS.md`](./AGENTS.md) — UI-first acceptance and conventions.
- [`../examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md) — CLI worker commands aligned with the UI.
