-- Federation agent registry (WP5 persistence). Applied via `sqlx::migrate!` when using Postgres memory URL.

CREATE TABLE IF NOT EXISTS federation_registry (
    agent_id TEXT PRIMARY KEY,
    capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_federation_registry_updated ON federation_registry (updated_at DESC);
