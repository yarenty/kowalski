-- WP3: SQLite — same logical model as migrations/postgres (default / simple single-node).
-- No pgvector equivalent here; use Qdrant (existing) or app-side vector search until sqlite-vec is added.
-- Apply via `kowalski_core::db::run_migrations("sqlite:…")` or sqlx migrate.

CREATE TABLE IF NOT EXISTS episodic_memory (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    session_id TEXT NOT NULL,
    ts TEXT NOT NULL DEFAULT (datetime('now')),
    role TEXT NOT NULL,
    content_text TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_episodic_session_ts ON episodic_memory (session_id, ts);

CREATE TABLE IF NOT EXISTS agent_state (
    agent_id TEXT PRIMARY KEY NOT NULL,
    current_task TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    capabilities TEXT
);
