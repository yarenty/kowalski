-- Remembered ("always allow") approvals — PostgreSQL parity with
-- `migrations/sqlite/005_standing_approvals.sql`.

CREATE TABLE IF NOT EXISTS standing_approval (
    id          BIGSERIAL PRIMARY KEY,
    horde_id    TEXT NOT NULL,
    step        TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    summary     TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    UNIQUE (horde_id, step, fingerprint)
);
