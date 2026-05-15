# Kowalski UI — agent notes

**Package**: `kowalski-ui` · **Version**: **1.2.0** (`package.json`)

## Role

Vue 3 + Vite + TypeScript single-page app. It is **not** the source of truth for business logic: the **`kowalski`** API defines behavior. This UI only consumes `/api/*`.

### UI-first (operator acceptance)

Features are **not done** until an operator can complete the primary flows in **`ui/`** against a running **`kowalski`** server:

- **Chat** tab (LLM + optional tools stream).
- **Rookery** tab (1.3.0): conversational horde builder → **Propose horde** → **PenguinCanvas** + **PenguinEditor** (save penguin) → **Give birth** → **Save horde to disk** (`/api/rookery/*`).
- **Horde** tab: catalog → worker lifecycle → **Horde Run** (e.g. Knowledge Compiler delivery).
- **Federation** tab: registry, worker start/stop, delegate smoke tests.

Backend or `kowalski-core` changes that touch chat, horde, federation, or delivery metadata **must** be smoke-checked here (or documented with a blocking reason). Error copy shown in panels should always reference **current** CLI commands (see [`examples/knowledge-compiler/README.md`](../examples/knowledge-compiler/README.md)), not deprecated wrappers.

## Conventions

- Prefer **`fetch`** and small composables; keep `App.vue` readable—extract new tabs into components if they grow.
- API helpers live in **`src/api.ts`**; extend `ChatStreamEvent` only when the backend adds event types.
- **Tool-aware stream**: checkbox binds to `chatToolsStream` and passes `{ toolsStream: true }` into `chatStream()`.

## Documentation closure (mandatory)

UI refactors (routes, API helpers, federation UX) must include updates to **[`README.md`](./README.md)**, **[`ROADMAP.md`](./ROADMAP.md)**, and root **[`CHANGELOG.md`](../CHANGELOG.md)** when operators see a change. Follow **Rule 7** in root [`../AGENTS.md`](../AGENTS.md).

## See also

- [`README.md`](./README.md) — includes **Operator smoke checklist (~2 minutes)** after backend/API sections.
- [`ROADMAP.md`](./ROADMAP.md) · root [`../AGENTS.md`](../AGENTS.md)
