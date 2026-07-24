-- Persisted horde-run store (durable-autonomy foundation): typed run/step state,
-- manifest snapshot at run start, per-step attempts, compact JSON events log.
-- Apply via `kowalski_core::db::run_migrations("sqlite:…")` or `RunStore::open*`.

CREATE TABLE IF NOT EXISTS horde_run (
    run_id TEXT PRIMARY KEY NOT NULL,
    horde_id TEXT NOT NULL,
    prompt TEXT NOT NULL DEFAULT '',
    source TEXT,
    question TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    current_step TEXT,
    -- JSON of the parsed horde spec, captured at run start (restart safety).
    manifest_snapshot TEXT,
    result TEXT,
    -- Compact JSON array of run events (appended via json_insert).
    events TEXT NOT NULL DEFAULT '[]',
    -- JSON object: loop-edge key -> traversal count (conditional DAG loops).
    loop_counts TEXT NOT NULL DEFAULT '{}',
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT
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
    started_at TEXT,
    finished_at TEXT,
    PRIMARY KEY (run_id, step)
);

CREATE INDEX IF NOT EXISTS idx_horde_run_step_status ON horde_run_step (status);
