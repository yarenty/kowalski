# Kowalski UI — agent notes

**Package**: `kowalski-ui` · **Version**: **1.1.0** (`package.json`)

## Role

Vue 3 + Vite + TypeScript single-page app. It is **not** the source of truth for business logic: the **`kowalski`** API defines behavior. This UI only consumes `/api/*`.

## Conventions

- Prefer **`fetch`** and small composables; keep `App.vue` readable—extract new tabs into components if they grow.
- API helpers live in **`src/api.ts`**; extend `ChatStreamEvent` only when the backend adds event types.
- **Tool-aware stream**: checkbox binds to `chatToolsStream` and passes `{ toolsStream: true }` into `chatStream()`.

## Documentation closure (mandatory)

UI refactors (routes, API helpers, federation UX) must include updates to **[`README.md`](./README.md)**, **[`ROADMAP.md`](./ROADMAP.md)**, and root **[`CHANGELOG.md`](../CHANGELOG.md)** when operators see a change. Follow **Rule 7** in root [`../AGENTS.md`](../AGENTS.md).

## See also

- [`README.md`](./README.md) · [`ROADMAP.md`](./ROADMAP.md) · root [`../AGENTS.md`](../AGENTS.md)
