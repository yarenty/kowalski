-- Resume support for interrupted horde runs — PostgreSQL parity with
-- `migrations/sqlite/004_run_resume.sql`.

ALTER TABLE horde_run ADD COLUMN IF NOT EXISTS origin TEXT NOT NULL DEFAULT 'operator';
ALTER TABLE horde_run ADD COLUMN IF NOT EXISTS resume_count INTEGER NOT NULL DEFAULT 0;
