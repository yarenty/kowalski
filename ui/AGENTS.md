# Kowalski UI — agent notes

**Package**: `kowalski-ui` · **Version**: **1.5.0** (`package.json`)

## Role

Vue 3 + Vite + TypeScript single-page app. It is **not** the source of truth for business logic: the **`kowalski`** API defines behavior. This UI only consumes `/api/*`.

### UI-first (operator acceptance)

Features are **not done** until an operator can complete the primary flows in **`ui/`** against a running **`kowalski`** server:

- **Chat** tab (LLM + optional tools stream).
- **Rookery** tab (1.3.0+): conversational horde builder → **Propose horde** → **PenguinCanvas** (linear track or layered DAG when `edges[]` present) + **PenguinEditor** (save penguin) → **Give birth** → **Save horde to disk** (`/api/rookery/*`).
- **Horde** tab: catalog → worker lifecycle → **Horde Run** (e.g. Knowledge Compiler delivery).
  The horde listing polls every 15 s (server catalog hot-reloads definitions — add/edit/remove
  without restart); a horde whose latest on-disk edit failed to parse shows a ⚠ badge and the
  `load_error` message while the server keeps running its last good version.
  The folded **Schedules and watchers** section of a horde page (when the horde declares
  `[[triggers]]`) lists each trigger with its kind/detail badge, on/off state, next cron fire
  and a link to the run it fired last; **Switch off / on** is a server-side operator override
  (persists across restarts, never edits `horde.md`) and **Run now** starts the trigger's run
  immediately. **Past runs of this horde** lists the last five runs by title; the **Runs** tab
  lists every run. Trigger definitions are **not** editable here — authoring stays in
  `horde.md` / Rookery.
- **Federation** tab: registry, worker start/stop, delegate smoke tests. Each horde card
  has an **Export** button (downloads the portable `<id>-<version>.kwf.zip` bundle via
  authenticated fetch — a plain link cannot carry the bearer header) and the hordes list
  has an **Import bundle…** action: the upload runs a server-side dry-run first
  (`POST /api/hordes/import?dry_run=true`), the portability report is shown, and only
  **Confirm import** lands the draft (triggers disabled) — the horde then appears through
  the catalog hot reload / 15 s poll without a restart. Invalid bundles surface the
  server's 400 message inline.

Backend or `kowalski-core` changes that touch chat, horde, federation, or delivery metadata **must** be smoke-checked here (or documented with a blocking reason). Error copy shown in panels should always reference **current** CLI commands (see [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md)), not deprecated wrappers.

## Conventions

- **Theme (Commando)**: every colour is a design token defined once on `:root` in
  `src/styles/theme.css` (light, dark via `prefers-color-scheme`, and `data-theme` overrides).
  Components use only `var(--…)` — no hex literals, gradients or glows. Shared building blocks
  (buttons, inputs, `.card`, `.badge`, `.chip`, `.note-*`, `.eyebrow`, `.page-head`,
  `.empty-state`) live there too; scoped styles only add layout. Keep text/background pairs
  WCAG AA.
- **Navigation labels**: the top bar shows **Hordes** (`federation-run`, default landing tab),
  **Runs** (`runs`), **Chat**, **Build** (`rookery`), **Setup**, and an **Admin ▾** menu
  (Federation, MCP servers, Graph, Diagnostics = `home`, About). Help text must use these names.
- **Routing**: `src/nav.ts` owns the tab ids and the URL mapping (`?tab=`, `?horde=`, `?run=`);
  `App.vue` pushes history entries so Back works. The Hordes tab shows the home (tiles) until a
  horde is opened, then `FederationRunPanel` for that horde (and run).
- **Shared horde list**: `src/hordesStore.ts` holds the catalogue for every screen (15 s poll
  while a screen uses it) — do not add per-panel `api.hordes()` loops.
- **Horde icons**: `src/hordeIcons.ts` is the single owner of categories (mirrors
  `HORDE_CATEGORIES` in `kowalski/src/horde.rs`), their `--cat-*` colour tokens and the icon
  paths; draw with `components/HordeIcon.vue`. Run status wording lives in `src/runs.ts`.
- **Runs across hordes** come from `GET /api/runs` (`api.runs()`): the Runs page, the ⌘K picker,
  the home's needs-you strip and tile status lines, and the horde page's past runs.

- Prefer **`fetch`** and small composables; keep `App.vue` readable—extract new tabs into components if they grow.
- API helpers live in **`src/api.ts`**; extend `ChatStreamEvent` only when the backend adds event types.
- **API token (only for servers started with `--auth` — auth is off by default):** `src/api.ts` sends `Authorization: Bearer <token>`
  on every call (`?token=` on `EventSource` URLs, which cannot set headers). The token comes
  from `localStorage` (`kowalski.api_token`, set via **Admin → Diagnostics → API token** or the first-run
  prompt in `App.vue`) with a `VITE_API_TOKEN` env fallback for dev. New fetch paths must go
  through `json()` / `streamSse` helpers or add `authHeaders()` themselves.
- **Tool-aware stream**: checkbox binds to `chatToolsStream` and passes `{ toolsStream: true }` into `chatStream()`.
- **Rookery draft is server-owned** (PLAN.md §R1): `localStorage` (`kowalski.ui.rookery.list.v1`) holds only a thin session list (`id`, `serverSessionId`, `title`, display `turns`, `updatedAt`). The draft/status/pipeline/summary/`edges` are **not** mirrored client-side — they are hydrated from `GET /api/rookery/sessions/{id}` on select/restore. Do not re-add a client-held draft round-trip to `POST /api/rookery/sessions`.
- **DAG layout:** `src/hordeGraph.ts` mirrors core layer scheduling for display only; orchestration stays on the server. Linear hordes (empty `edges`) keep the horizontal canvas.

## Documentation closure (mandatory)

UI refactors (routes, API helpers, federation UX) must include updates to **[`README.md`](./README.md)**, **[`ROADMAP.md`](./ROADMAP.md)**, and root **[`CHANGELOG.md`](../CHANGELOG.md)** when operators see a change. Follow **Rule 7** in root [`../AGENTS.md`](../AGENTS.md).

## See also

- [`README.md`](./README.md) — includes **Operator smoke checklist (~2 minutes)** after backend/API sections.
- [`ROADMAP.md`](./ROADMAP.md) · root [`../AGENTS.md`](../AGENTS.md)
