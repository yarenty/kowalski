-- Tier 2 episodic buffer: same JSON payload model as SQLite `episodic_kv`.
-- Used when agents store episodic memory in PostgreSQL (`memory.database_url`).

CREATE TABLE IF NOT EXISTS episodic_kv (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL
);
