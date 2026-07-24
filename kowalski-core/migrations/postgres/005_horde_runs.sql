-- Persisted horde-run store — PostgreSQL parity with `migrations/sqlite/003_horde_runs.sql`.
-- The default runs backend is the zero-config SQLite file; this keeps the optional
-- Postgres deployment able to host the same schema.

CREATE TABLE IF NOT EXISTS horde_run (
    run_id TEXT PRIMARY KEY NOT NULL,
    horde_id TEXT NOT NULL,
    prompt TEXT NOT NULL DEFAULT '',
    source TEXT,
    question TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    current_step TEXT,
    -- JSON of the parsed horde spec, captured at run start (restart safety).
    manifest_snapshot JSONB,
    result TEXT,
    -- Compact JSON array of run events.
    events JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- JSON object: loop-edge key -> traversal count (conditional DAG loops).
    loop_counts JSONB NOT NULL DEFAULT '{}'::jsonb,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_horde_run_status ON horde_run (status);
CREATE INDEX IF NOT EXISTS idx_horde_run_horde_started ON horde_run (horde_id, started_at);

CREATE TABLE IF NOT EXISTS horde_run_step (
    run_id TEXT NOT NULL REFERENCES horde_run(run_id) ON DELETE CASCADE,
    step TEXT NOT NULL,
    agent_id TEXT NOT NULL DEFAULT '',
    task_id TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    attempt INTEGER NOT NULL DEFAULT 1,
    -- `pass` / `fail` from verify-style artifacts (drives conditional edges).
    outcome TEXT,
    -- Filesystem path of the step artifact under the horde workdir.
    artifact TEXT,
    summary TEXT,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    PRIMARY KEY (run_id, step)
);

CREATE INDEX IF NOT EXISTS idx_horde_run_step_status ON horde_run_step (status);
