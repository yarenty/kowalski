-- Resume support for interrupted horde runs: how the run was started (resume
-- policy input: non-operator runs may auto-resume on server startup) and how
-- many resume attempts were spent (guard rail before the run goes to error).

ALTER TABLE horde_run ADD COLUMN origin TEXT NOT NULL DEFAULT 'operator';
ALTER TABLE horde_run ADD COLUMN resume_count INTEGER NOT NULL DEFAULT 0;
