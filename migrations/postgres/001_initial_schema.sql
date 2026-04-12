-- WP3: PostgreSQL — episodic log + agent registry (vector semantic table is a later migration).
-- Apply: psql "$DATABASE_URL" -f migrations/postgres/001_initial_schema.sql

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS episodic_memory (
    id BIGSERIAL PRIMARY KEY,
    session_id TEXT NOT NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    role TEXT NOT NULL,
    content_text TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_episodic_session_ts ON episodic_memory (session_id, ts);

CREATE TABLE IF NOT EXISTS agent_state (
    agent_id TEXT PRIMARY KEY,
    current_task TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    capabilities JSONB
);
