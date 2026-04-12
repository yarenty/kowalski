-- Key-value store for Tier 2 episodic buffer: full MemoryUnit JSON in `payload`.
-- Used by `EpisodicBuffer` (separate SQLite file from optional `memory.database_url`).

CREATE TABLE IF NOT EXISTS episodic_kv (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL
);
