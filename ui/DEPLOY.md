# Production deploy (Vue operator UI)

## Build

From `ui/`:

```bash
bun install
bun run build
```

Artifacts land in `dist/`. Serve `dist/` as static files from any CDN or object storage (SPA: fallback to `index.html` for client routes if you add any).

## API base URL

The dev server proxies `/api` to `kowalski` (see `vite.config.ts`). In production, set **`VITE_API_BASE`** to the **origin** of the HTTP API **before** `bun run build`, for example:

```bash
VITE_API_BASE=https://kowalski.example.com bun run build
```

If the UI and API share the same origin, you can use an empty base or `/` per `api.ts` conventions.

## CORS

`kowalski` uses permissive CORS for local dev. For a locked-down production split (UI on CDN, API on another host), restrict CORS in a reverse proxy or extend the Axum layer as needed.
