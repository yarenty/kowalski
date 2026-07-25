//! Persisted horde-run store: typed run/step state machine over SQLite.
//!
//! Foundation for durable runs — the orchestrator writes through this store so
//! in-flight runs survive a server restart. Zero config by default: the store
//! creates `runs.sqlite` under the server state dir (see [`RunStore::open_default`];
//! override with the `KOWALSKI_RUN_DB` env var, a full `sqlite:` URL).
//!
//! Schema: `migrations/sqlite/003_horde_runs.sql` (PostgreSQL parity in
//! `migrations/postgres/005_horde_runs.sql`).

use crate::error::KowalskiError;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

/// Default file name of the run-store database under the server state dir.
pub const RUN_DB_FILE_NAME: &str = "runs.sqlite";

/// Default `origin` for runs started by an operator (UI or manual API call).
/// Non-operator origins (e.g. `"trigger"`) may be auto-resumed on server startup.
pub const RUN_ORIGIN_OPERATOR: &str = "operator";

/// Run lifecycle. `AwaitingInput` is durable by construction: waiting for operator
/// input ends the executor task, so such runs survive restarts untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    AwaitingInput,
    Done,
    Error,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Pending => "pending",
            RunStatus::Running => "running",
            RunStatus::AwaitingInput => "awaiting_input",
            RunStatus::Done => "done",
            RunStatus::Error => "error",
            RunStatus::Cancelled => "cancelled",
        }
    }

    /// Statuses a restart scan must pick up (the run did not reach a terminal state).
    pub fn is_incomplete(&self) -> bool {
        matches!(
            self,
            RunStatus::Pending | RunStatus::Running | RunStatus::AwaitingInput
        )
    }
}

impl FromStr for RunStatus {
    type Err = KowalskiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(RunStatus::Pending),
            "running" => Ok(RunStatus::Running),
            "awaiting_input" => Ok(RunStatus::AwaitingInput),
            "done" => Ok(RunStatus::Done),
            "error" => Ok(RunStatus::Error),
            "cancelled" => Ok(RunStatus::Cancelled),
            other => Err(KowalskiError::Configuration(format!(
                "unknown run status: {other}"
            ))),
        }
    }
}

/// Per-step lifecycle within a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Delegating,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::Delegating => "delegating",
            StepStatus::Running => "running",
            StepStatus::Succeeded => "succeeded",
            StepStatus::Failed => "failed",
            StepStatus::Cancelled => "cancelled",
            StepStatus::Skipped => "skipped",
        }
    }
}

impl FromStr for StepStatus {
    type Err = KowalskiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(StepStatus::Pending),
            "delegating" => Ok(StepStatus::Delegating),
            "running" => Ok(StepStatus::Running),
            "succeeded" => Ok(StepStatus::Succeeded),
            "failed" => Ok(StepStatus::Failed),
            "cancelled" => Ok(StepStatus::Cancelled),
            "skipped" => Ok(StepStatus::Skipped),
            other => Err(KowalskiError::Configuration(format!(
                "unknown step status: {other}"
            ))),
        }
    }
}

/// Input for [`RunStore::create_run`]: identity + operator input + the manifest
/// snapshot captured at run start (restart safety).
#[derive(Debug, Clone)]
pub struct NewRun {
    pub run_id: String,
    pub horde_id: String,
    pub prompt: String,
    pub source: Option<String>,
    pub question: String,
    pub manifest_snapshot: Option<serde_json::Value>,
    /// How the run was started ([`RUN_ORIGIN_OPERATOR`] for UI/API operators;
    /// non-operator origins may be auto-resumed on server startup).
    pub origin: String,
}

/// Persisted run row (steps loaded alongside via [`RunStore::get_run`]).
#[derive(Debug, Clone, Serialize)]
pub struct PersistedRun {
    pub run_id: String,
    pub horde_id: String,
    pub prompt: String,
    pub source: Option<String>,
    pub question: String,
    pub status: RunStatus,
    pub current_step: Option<String>,
    pub manifest_snapshot: Option<serde_json::Value>,
    pub result: Option<String>,
    pub origin: String,
    /// Resume attempts spent so far (guard rail: the orchestrator errors the run
    /// once this exceeds its configured cap).
    pub resume_count: i64,
    pub events: Vec<serde_json::Value>,
    pub loop_counts: BTreeMap<String, u32>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub steps: Vec<PersistedRunStep>,
}

/// Persisted step row; one row per (run, step), `attempt` counts loop re-runs.
#[derive(Debug, Clone, Serialize)]
pub struct PersistedRunStep {
    pub step: String,
    pub agent_id: String,
    pub task_id: String,
    pub status: StepStatus,
    pub attempt: i64,
    pub outcome: Option<String>,
    pub artifact: Option<String>,
    pub summary: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

/// Field set written by [`RunStore::upsert_step`].
#[derive(Debug, Clone)]
pub struct StepUpdate {
    pub step: String,
    pub agent_id: String,
    pub task_id: String,
    pub status: StepStatus,
    pub attempt: i64,
    pub outcome: Option<String>,
    pub artifact: Option<String>,
    pub summary: Option<String>,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn db_err(e: sqlx::Error) -> KowalskiError {
    super::db_err(e)
}

/// SQLite-backed store for horde runs. Cheap to clone (pool handle).
#[derive(Clone)]
pub struct RunStore {
    pool: SqlitePool,
}

impl RunStore {
    /// Open (and migrate) the store at `database_url` (`sqlite:` URLs only).
    pub async fn open(database_url: &str) -> Result<Self, KowalskiError> {
        if !database_url.starts_with("sqlite:") {
            return Err(KowalskiError::Configuration(format!(
                "run store requires a sqlite: URL, got: {database_url}"
            )));
        }
        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(db_err)?
            .create_if_missing(true);
        Self::open_with(options, database_url.contains(":memory:")).await
    }

    /// Zero-config open: `KOWALSKI_RUN_DB` env URL if set, else
    /// `<state_dir>/runs.sqlite` (created together with `state_dir` if missing).
    pub async fn open_default(state_dir: &Path) -> Result<Self, KowalskiError> {
        if let Ok(env) = std::env::var(crate::config::RUN_DB_ENV) {
            let env = env.trim().to_string();
            if !env.is_empty() {
                return Self::open(&env).await;
            }
        }
        std::fs::create_dir_all(state_dir).map_err(|e| {
            KowalskiError::Configuration(format!(
                "run store dir {}: {e}",
                state_dir.display()
            ))
        })?;
        let options = SqliteConnectOptions::new()
            .filename(state_dir.join(RUN_DB_FILE_NAME))
            .create_if_missing(true);
        Self::open_with(options, false).await
    }

    /// `in_memory`: an in-memory SQLite DB lives inside ONE connection — pin the pool
    /// to a single never-recycled connection or every checkout sees an empty schema.
    async fn open_with(
        options: SqliteConnectOptions,
        in_memory: bool,
    ) -> Result<Self, KowalskiError> {
        let mut pool_options = SqlitePoolOptions::new();
        if in_memory {
            pool_options = pool_options
                .max_connections(1)
                .idle_timeout(None)
                .max_lifetime(None);
        }
        let pool = pool_options
            .connect_with(options)
            .await
            .map_err(db_err)?;
        super::SQLITE_MIGRATOR
            .run(&pool)
            .await
            .map_err(super::migrate_err)?;
        Ok(Self { pool })
    }

    pub async fn create_run(&self, run: &NewRun) -> Result<(), KowalskiError> {
        let snapshot = run
            .manifest_snapshot
            .as_ref()
            .map(|v| v.to_string());
        sqlx::query(
            "INSERT INTO horde_run (run_id, horde_id, prompt, source, question, status, manifest_snapshot, origin, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?7, ?8)",
        )
        .bind(&run.run_id)
        .bind(&run.horde_id)
        .bind(&run.prompt)
        .bind(&run.source)
        .bind(&run.question)
        .bind(snapshot)
        .bind(&run.origin)
        .bind(now_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    /// Update run status (+ optional result text); terminal statuses stamp `finished_at`.
    pub async fn update_run_status(
        &self,
        run_id: &str,
        status: RunStatus,
        result: Option<&str>,
    ) -> Result<(), KowalskiError> {
        let finished_at = (!status.is_incomplete()).then(now_rfc3339);
        sqlx::query(
            "UPDATE horde_run
             SET status = ?1,
                 result = COALESCE(?2, result),
                 finished_at = COALESCE(?3, finished_at)
             WHERE run_id = ?4",
        )
        .bind(status.as_str())
        .bind(result)
        .bind(finished_at)
        .bind(run_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    pub async fn set_current_step(
        &self,
        run_id: &str,
        step: Option<&str>,
    ) -> Result<(), KowalskiError> {
        sqlx::query("UPDATE horde_run SET current_step = ?1 WHERE run_id = ?2")
            .bind(step)
            .bind(run_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    pub async fn set_loop_counts(
        &self,
        run_id: &str,
        loop_counts: &BTreeMap<String, u32>,
    ) -> Result<(), KowalskiError> {
        let json = serde_json::to_string(loop_counts)
            .map_err(|e| KowalskiError::Configuration(format!("loop_counts: {e}")))?;
        sqlx::query("UPDATE horde_run SET loop_counts = ?1 WHERE run_id = ?2")
            .bind(json)
            .bind(run_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// Insert or update one step row. First transition out of `pending` stamps
    /// `started_at`; terminal statuses stamp `finished_at`.
    pub async fn upsert_step(&self, run_id: &str, step: &StepUpdate) -> Result<(), KowalskiError> {
        let now = now_rfc3339();
        let started_at = (step.status != StepStatus::Pending).then(|| now.clone());
        let finished_at = matches!(
            step.status,
            StepStatus::Succeeded | StepStatus::Failed | StepStatus::Cancelled | StepStatus::Skipped
        )
        .then(|| now.clone());
        sqlx::query(
            "INSERT INTO horde_run_step
                 (run_id, step, agent_id, task_id, status, attempt, outcome, artifact, summary, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT (run_id, step) DO UPDATE SET
                 agent_id = excluded.agent_id,
                 task_id = excluded.task_id,
                 status = excluded.status,
                 attempt = excluded.attempt,
                 outcome = excluded.outcome,
                 artifact = excluded.artifact,
                 summary = excluded.summary,
                 started_at = COALESCE(horde_run_step.started_at, excluded.started_at),
                 finished_at = excluded.finished_at",
        )
        .bind(run_id)
        .bind(&step.step)
        .bind(&step.agent_id)
        .bind(&step.task_id)
        .bind(step.status.as_str())
        .bind(step.attempt)
        .bind(&step.outcome)
        .bind(&step.artifact)
        .bind(&step.summary)
        .bind(started_at)
        .bind(finished_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    /// Spend one resume attempt on the run; returns the new total (atomic).
    pub async fn increment_resume_count(&self, run_id: &str) -> Result<i64, KowalskiError> {
        let row = sqlx::query(
            "UPDATE horde_run SET resume_count = resume_count + 1
             WHERE run_id = ?1
             RETURNING resume_count",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.map(|r| r.get("resume_count")).ok_or_else(|| {
            KowalskiError::Configuration(format!("run {run_id} not found"))
        })
    }

    /// Append one event to the run's compact JSON events log (atomic, no read-modify-write).
    pub async fn record_event(
        &self,
        run_id: &str,
        event: &serde_json::Value,
    ) -> Result<(), KowalskiError> {
        sqlx::query(
            "UPDATE horde_run SET events = json_insert(events, '$[#]', json(?1)) WHERE run_id = ?2",
        )
        .bind(event.to_string())
        .bind(run_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Option<PersistedRun>, KowalskiError> {
        let row = sqlx::query("SELECT * FROM horde_run WHERE run_id = ?1")
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_err)?;
        let Some(row) = row else { return Ok(None) };
        let mut run = run_from_row(&row)?;
        run.steps = self.steps_for(run_id).await?;
        Ok(Some(run))
    }

    /// Runs newest-first, optionally filtered by horde, paged via limit/offset.
    /// Steps are not loaded here — use [`RunStore::get_run`] for the full record.
    pub async fn list_runs(
        &self,
        horde_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PersistedRun>, KowalskiError> {
        let rows = sqlx::query(
            "SELECT * FROM horde_run
             WHERE (?1 IS NULL OR horde_id = ?1)
             ORDER BY started_at DESC, run_id DESC
             LIMIT ?2 OFFSET ?3",
        )
        .bind(horde_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(run_from_row).collect()
    }

    /// Runs a restart scan must reconcile: `pending`, `running`, or `awaiting_input`.
    pub async fn incomplete_runs(&self) -> Result<Vec<PersistedRun>, KowalskiError> {
        let rows = sqlx::query(
            "SELECT * FROM horde_run
             WHERE status IN ('pending', 'running', 'awaiting_input')
             ORDER BY started_at ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(run_from_row).collect()
    }

    async fn steps_for(&self, run_id: &str) -> Result<Vec<PersistedRunStep>, KowalskiError> {
        let rows = sqlx::query(
            "SELECT * FROM horde_run_step WHERE run_id = ?1 ORDER BY started_at ASC, step ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter()
            .map(|row| {
                Ok(PersistedRunStep {
                    step: row.get("step"),
                    agent_id: row.get("agent_id"),
                    task_id: row.get("task_id"),
                    status: row.get::<String, _>("status").parse()?,
                    attempt: row.get("attempt"),
                    outcome: row.get("outcome"),
                    artifact: row.get("artifact"),
                    summary: row.get("summary"),
                    started_at: row.get("started_at"),
                    finished_at: row.get("finished_at"),
                })
            })
            .collect()
    }
}

fn run_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<PersistedRun, KowalskiError> {
    let manifest_snapshot = row
        .get::<Option<String>, _>("manifest_snapshot")
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| KowalskiError::Configuration(format!("manifest_snapshot: {e}")))?;
    let events: Vec<serde_json::Value> =
        serde_json::from_str(&row.get::<String, _>("events"))
            .map_err(|e| KowalskiError::Configuration(format!("events: {e}")))?;
    let loop_counts: BTreeMap<String, u32> =
        serde_json::from_str(&row.get::<String, _>("loop_counts"))
            .map_err(|e| KowalskiError::Configuration(format!("loop_counts: {e}")))?;
    Ok(PersistedRun {
        run_id: row.get("run_id"),
        horde_id: row.get("horde_id"),
        prompt: row.get("prompt"),
        source: row.get("source"),
        question: row.get("question"),
        status: row.get::<String, _>("status").parse()?,
        current_step: row.get("current_step"),
        manifest_snapshot,
        result: row.get("result"),
        origin: row.get("origin"),
        resume_count: row.get("resume_count"),
        events,
        loop_counts,
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        steps: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn new_run(id: &str, horde: &str) -> NewRun {
        NewRun {
            run_id: id.into(),
            horde_id: horde.into(),
            prompt: "prompt".into(),
            source: Some("https://example.com".into()),
            question: "q?".into(),
            manifest_snapshot: Some(json!({"pipeline": ["a", "b"], "edges": []})),
            origin: RUN_ORIGIN_OPERATOR.into(),
        }
    }

    async fn memory_store() -> RunStore {
        RunStore::open("sqlite::memory:").await.unwrap()
    }

    #[test]
    fn status_round_trips() {
        for s in [
            RunStatus::Pending,
            RunStatus::Running,
            RunStatus::AwaitingInput,
            RunStatus::Done,
            RunStatus::Error,
            RunStatus::Cancelled,
        ] {
            assert_eq!(s.as_str().parse::<RunStatus>().unwrap(), s);
        }
        for s in [
            StepStatus::Pending,
            StepStatus::Delegating,
            StepStatus::Running,
            StepStatus::Succeeded,
            StepStatus::Failed,
            StepStatus::Cancelled,
            StepStatus::Skipped,
        ] {
            assert_eq!(s.as_str().parse::<StepStatus>().unwrap(), s);
        }
        assert!("bogus".parse::<RunStatus>().is_err());
        assert!("bogus".parse::<StepStatus>().is_err());
    }

    #[tokio::test]
    async fn full_state_transition_cycle() {
        let store = memory_store().await;
        store.create_run(&new_run("r1", "h1")).await.unwrap();

        store
            .update_run_status("r1", RunStatus::Running, None)
            .await
            .unwrap();
        store.set_current_step("r1", Some("a")).await.unwrap();
        for (status, outcome) in [
            (StepStatus::Delegating, None),
            (StepStatus::Running, None),
            (StepStatus::Succeeded, Some("pass".to_string())),
        ] {
            store
                .upsert_step(
                    "r1",
                    &StepUpdate {
                        step: "a".into(),
                        agent_id: "agent-a".into(),
                        task_id: "t1".into(),
                        status,
                        attempt: 1,
                        outcome: outcome.clone(),
                        artifact: outcome.is_some().then(|| "out/a.md".into()),
                        summary: None,
                    },
                )
                .await
                .unwrap();
        }
        store.record_event("r1", &json!({"kind": "step_finished", "step": "a"})).await.unwrap();
        store
            .set_loop_counts("r1", &BTreeMap::from([("b->a".to_string(), 1)]))
            .await
            .unwrap();
        store
            .update_run_status("r1", RunStatus::Done, Some("all good"))
            .await
            .unwrap();

        let run = store.get_run("r1").await.unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Done);
        assert_eq!(run.result.as_deref(), Some("all good"));
        assert_eq!(run.current_step.as_deref(), Some("a"));
        assert!(run.finished_at.is_some());
        assert_eq!(run.events.len(), 1);
        assert_eq!(run.loop_counts.get("b->a"), Some(&1));
        assert_eq!(run.steps.len(), 1);
        let step = &run.steps[0];
        assert_eq!(step.status, StepStatus::Succeeded);
        assert_eq!(step.outcome.as_deref(), Some("pass"));
        assert_eq!(step.artifact.as_deref(), Some("out/a.md"));
        assert!(step.started_at.is_some() && step.finished_at.is_some());
    }

    #[tokio::test]
    async fn origin_and_resume_count_round_trip() {
        let store = memory_store().await;
        store.create_run(&new_run("r1", "h1")).await.unwrap();
        let mut trigger = new_run("r2", "h1");
        trigger.origin = "trigger".into();
        store.create_run(&trigger).await.unwrap();

        let r1 = store.get_run("r1").await.unwrap().unwrap();
        assert_eq!(r1.origin, RUN_ORIGIN_OPERATOR);
        assert_eq!(r1.resume_count, 0);
        let r2 = store.get_run("r2").await.unwrap().unwrap();
        assert_eq!(r2.origin, "trigger");

        assert_eq!(store.increment_resume_count("r1").await.unwrap(), 1);
        assert_eq!(store.increment_resume_count("r1").await.unwrap(), 2);
        assert_eq!(store.get_run("r1").await.unwrap().unwrap().resume_count, 2);
        assert!(store.increment_resume_count("missing").await.is_err());
    }

    #[tokio::test]
    async fn manifest_snapshot_round_trips() {
        let store = memory_store().await;
        let run = new_run("r1", "h1");
        store.create_run(&run).await.unwrap();
        let loaded = store.get_run("r1").await.unwrap().unwrap();
        assert_eq!(loaded.manifest_snapshot, run.manifest_snapshot);
    }

    #[tokio::test]
    async fn incomplete_runs_filters_terminal_statuses() {
        let store = memory_store().await;
        for (id, status) in [
            ("r1", RunStatus::Done),
            ("r2", RunStatus::Running),
            ("r3", RunStatus::AwaitingInput),
            ("r4", RunStatus::Error),
            ("r5", RunStatus::Cancelled),
        ] {
            store.create_run(&new_run(id, "h1")).await.unwrap();
            store.update_run_status(id, status, None).await.unwrap();
        }
        let incomplete = store.incomplete_runs().await.unwrap();
        let ids: Vec<&str> = incomplete.iter().map(|r| r.run_id.as_str()).collect();
        assert_eq!(ids, vec!["r2", "r3"]);
    }

    #[tokio::test]
    async fn list_runs_pages_newest_first() {
        let store = memory_store().await;
        store.create_run(&new_run("r1", "h1")).await.unwrap();
        store.create_run(&new_run("r2", "h1")).await.unwrap();
        store.create_run(&new_run("r3", "h2")).await.unwrap();

        let all = store.list_runs(None, 10, 0).await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].run_id, "r3");
        let h1 = store.list_runs(Some("h1"), 10, 0).await.unwrap();
        assert_eq!(h1.len(), 2);
        let paged = store.list_runs(None, 1, 1).await.unwrap();
        assert_eq!(paged.len(), 1);
        assert_eq!(paged[0].run_id, "r2");
    }

    #[tokio::test]
    async fn default_path_creates_file_db() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("db");
        let store = RunStore::open_default(&state_dir).await.unwrap();
        store.create_run(&new_run("r1", "h1")).await.unwrap();
        assert!(state_dir.join(RUN_DB_FILE_NAME).exists());
        let reopened = RunStore::open_default(&state_dir).await.unwrap();
        assert!(reopened.get_run("r1").await.unwrap().is_some());
    }
}
