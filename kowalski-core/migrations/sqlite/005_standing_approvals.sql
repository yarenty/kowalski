-- Remembered ("always allow") approvals for command steps. One row covers one step of one
-- horde doing one exact thing: `fingerprint` is what the operator approved (step kind,
-- command, folder); when the horde's definition changes it, the step asks again.

CREATE TABLE IF NOT EXISTS standing_approval (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    horde_id    TEXT NOT NULL,
    step        TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    summary     TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    UNIQUE (horde_id, step, fingerprint)
);
