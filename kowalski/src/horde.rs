//! Horde catalog, durable run state, and server-side orchestrator task.
//!
//! A *horde* is a markdown-defined multi-agent workflow with one worker per sub-agent.
//! The orchestrator subscribes to the federation broker, advances the run pipeline as each
//! sub-agent's worker reports a [`crate::core::AclMessage::TaskFinished`], and emits
//! lifecycle events (`RunStarted`, `TaskAssigned`, `AgentMessage`, `RunFinished`, `RunFailed`).
//!
//! Run state is written through to the persisted run store
//! ([`kowalski_core::db::run_store::RunStore`]) on every transition — run created (with a
//! manifest snapshot of the loaded [`HordeSpec`]), step delegating/succeeded/failed, loop
//! counts, run done/error — so runs survive a server restart. The in-memory
//! [`RunRegistry`] is a cache for the active-run hot path; `/api` reads go to the store.

use kowalski_core::MessageBroker;
use kowalski_core::db::run_store::{
    NewRun, PersistedRun, RUN_ORIGIN_OPERATOR, RunStatus, RunStore, StepStatus, StepUpdate,
};
use kowalski_core::federation::{AclEnvelope, AclMessage, FederationOrchestrator, MpscBroker};
use kowalski_core::{
    all_steps_successful, has_conditional_outbound, is_loop_back_step, loop_edge_key,
    next_ready_step_conditional, parse_stage_status_from_artifact, resolve_execution_graph,
    retry_span, select_next_from_outcome, single_predecessor, verify_output_excerpt, StageStatus,
    ExecutionGraph, HordeEdge,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const DEFAULT_TOPIC: &str = "federation";
/// Default cap on resume attempts per run; a run whose resumes keep failing goes
/// to `error` with a reason instead of looping forever. Override with
/// `[horde] resume_max_attempts` in `config.toml`.
pub const DEFAULT_RESUME_MAX_ATTEMPTS: u32 = 2;
/// Relative path under `workdir` for managed federation worker stdout/stderr logs (HTTP server convention).
pub const AGENTS_LOG_REL: &str = "agents_log";
/// Relative path under `workdir` for follow-up chat markdown from `POST .../followup` (HTTP server convention).
pub const FOLLOWUP_ARTIFACT_REL: &str = "debug/followups";
static RUN_SEQ: AtomicU64 = AtomicU64::new(1);

fn now_ts() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => format!("{}.{:03}Z", d.as_secs(), d.subsec_millis()),
        Err(_) => "0.000Z".to_string(),
    }
}

fn new_run_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let seq = RUN_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("run-{}-{}", ms, seq)
}

#[derive(Debug, Deserialize)]
pub struct HordeMeta {
    pub id: String,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub capability_prefix: Option<String>,
    pub pipeline: Vec<String>,
    #[serde(default)]
    pub edges: Vec<HordeEdge>,
    #[serde(default)]
    pub default_question: Option<String>,
    #[serde(default)]
    pub default_topic: Option<String>,
    #[serde(default)]
    pub artifacts_root: Option<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    #[serde(alias = "clean_on_startup")]
    pub config_on_startup: Option<bool>,
    #[serde(default)]
    pub delivery_title: Option<String>,
    #[serde(default)]
    pub delivery_note: Option<String>,
    #[serde(default)]
    pub delivery_root_rel: Option<String>,
    #[serde(default)]
    pub delivery_summary_note: Option<String>,
    #[serde(default)]
    pub prompt_tip: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubAgentMeta {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub default_agent_id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub prompt_file: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub inputs: Vec<kowalski_core::OperatorInputField>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub tool_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubAgentSpec {
    pub name: String,
    pub kind: String,
    pub capability: String,
    pub default_agent_id: String,
    pub display_name: String,
    pub description: String,
    pub prompt_file: Option<String>,
    pub output: Option<String>,
    pub inputs: Vec<kowalski_core::OperatorInputField>,
    pub avatar: Option<String>,
    pub tool_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HordeSpec {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub capability_prefix: String,
    pub pipeline: Vec<String>,
    /// Explicit `[[edges]]` from manifest (empty = linear horde).
    pub manifest_edges: Vec<HordeEdge>,
    #[serde(skip)]
    pub execution_graph: ExecutionGraph,
    pub default_question: String,
    pub topic: String,
    pub artifacts_root: PathBuf,
    pub workdir: PathBuf,
    pub config_on_startup: bool,
    pub delivery_title: String,
    pub delivery_note: String,
    pub delivery_root_rel: String,
    pub delivery_summary_note: String,
    pub prompt_tip: String,
    pub root_path: PathBuf,
    pub sub_agents: Vec<SubAgentSpec>,
    /// Resolved directory for follow-up chat artifacts ([`FOLLOWUP_ARTIFACT_REL`] under `workdir`).
    pub followup_artifact_dir: PathBuf,
    /// Resolved directory for managed worker process logs ([`AGENTS_LOG_REL`] under `workdir`).
    pub worker_log_dir: PathBuf,
    /// Pre-run operator form (first pipeline step that declares `[[inputs]]`).
    pub run_form: Option<kowalski_core::HordeRunFormSpec>,
}

impl HordeSpec {
    pub fn sub_agent(&self, name: &str) -> Option<&SubAgentSpec> {
        self.sub_agents.iter().find(|a| a.name == name)
    }
}

fn parse_md_with_toml<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let mut lines = raw.lines();
    if lines.next().map(|s| s.trim()) != Some("---") {
        return Err(format!("Missing frontmatter in {}", path.display()).into());
    }
    let mut fm = String::new();
    let mut in_fm = true;
    for line in raw.lines().skip(1) {
        if in_fm && line.trim() == "---" {
            in_fm = false;
            break;
        }
        if in_fm {
            fm.push_str(line);
            fm.push('\n');
        }
    }
    if in_fm {
        return Err(format!("Unterminated frontmatter in {}", path.display()).into());
    }
    Ok(toml::from_str::<T>(&fm)?)
}

pub fn load_horde(root: &Path) -> Result<HordeSpec, Box<dyn std::error::Error>> {
    let manifest_path = root.join("horde.md");
    let meta: HordeMeta = parse_md_with_toml(&manifest_path)?;
    let prefix = meta
        .capability_prefix
        .clone()
        .unwrap_or_else(|| meta.id.clone());

    let agents_dir = root.join("agents");
    if !agents_dir.is_dir() {
        return Err(format!("agents/ missing under {}", root.display()).into());
    }
    let mut by_name: HashMap<String, SubAgentSpec> = HashMap::new();
    for entry in std::fs::read_dir(&agents_dir)? {
        let p = entry?.path();
        if p.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let raw: SubAgentMeta = parse_md_with_toml(&p)?;
        let capability = raw
            .capability
            .clone()
            .unwrap_or_else(|| format!("{}.{}", prefix, raw.kind));
        let default_agent_id = raw
            .default_agent_id
            .clone()
            .unwrap_or_else(|| format!("{}-{}", prefix.replace('.', "-"), raw.kind));
        let display_name = raw.display_name.clone().unwrap_or_else(|| {
            let mut s = raw.kind.clone();
            if let Some(c) = s.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            format!("{} Agent", s)
        });
        let description = raw
            .description
            .clone()
            .unwrap_or_else(|| format!("{} sub-agent of {}", raw.kind, meta.id));
        let avatar = raw.avatar.clone().or_else(|| {
            Some(kowalski_core::infer_penguin_avatar(&raw.kind, &raw.name))
        });
        by_name.insert(
            raw.name.clone(),
            SubAgentSpec {
                name: raw.name,
                kind: raw.kind,
                capability,
                default_agent_id,
                display_name,
                description,
                prompt_file: raw.prompt_file,
                output: raw.output,
                inputs: raw.inputs,
                avatar,
                tool_ids: raw.tool_ids,
            },
        );
    }

    let mut sub_agents = Vec::new();
    for name in &meta.pipeline {
        let agent = by_name
            .remove(name)
            .ok_or_else(|| format!("pipeline references missing sub-agent `{}`", name))?;
        sub_agents.push(agent);
    }

    let workdir = if let Some(w) = &meta.workdir {
        let p = PathBuf::from(w.clone());
        if p.is_absolute() {
            p
        } else {
            root.join(w)
        }
    } else {
        root.join("workdir")
    };

    let followup_artifact_dir = workdir.join(FOLLOWUP_ARTIFACT_REL);
    let worker_log_dir = workdir.join(AGENTS_LOG_REL);

    let edge_slice = if meta.edges.is_empty() {
        None
    } else {
        Some(meta.edges.as_slice())
    };
    let execution_graph = resolve_execution_graph(&meta.pipeline, edge_slice)
        .map_err(|e| e.to_string())?;

    let run_form = sub_agents
        .iter()
        .find(|a| !a.inputs.is_empty())
        .map(|a| kowalski_core::HordeRunFormSpec {
            step: a.name.clone(),
            display_name: Some(a.display_name.clone()),
            inputs: a.inputs.clone(),
        });

    Ok(HordeSpec {
        id: meta.id,
        display_name: meta.display_name,
        description: meta.description,
        capability_prefix: prefix,
        pipeline: meta.pipeline.clone(),
        manifest_edges: meta.edges.clone(),
        execution_graph,
        default_question: meta
            .default_question
            .unwrap_or_else(|| "What changed?".to_string()),
        topic: meta.default_topic.unwrap_or_else(|| DEFAULT_TOPIC.to_string()),
        artifacts_root: root.join(meta.artifacts_root.unwrap_or_else(|| ".".to_string())),
        workdir,
        config_on_startup: meta.config_on_startup.unwrap_or(false),
        delivery_title: meta
            .delivery_title
            .unwrap_or_else(|| "Final delivery".to_string()),
        delivery_note: meta.delivery_note.unwrap_or_else(|| {
            "When the run completes, use the markdown hand-off in the run payload (if present) or the file named by `delivery_root_rel` under the workdir. Intermediate artifacts are usually under `workdir/debug/`."
                .to_string()
        }),
        delivery_root_rel: {
            let r = meta
                .delivery_root_rel
                .unwrap_or_else(|| "HANDOFF.md".to_string());
            if kowalski_core::rookery::output_looks_invalid(&r) {
                "HANDOFF.md".to_string()
            } else {
                r
            }
        },
        delivery_summary_note: meta
            .delivery_summary_note
            .unwrap_or_default(),
        prompt_tip: meta.prompt_tip.unwrap_or_else(|| {
            "Provide a prompt that includes source URL and desired output style.".to_string()
        }),
        root_path: root.to_path_buf(),
        sub_agents,
        followup_artifact_dir,
        worker_log_dir,
        run_form,
    })
}

/// Remove horde workdir artifacts (same tree as “clean on startup”): `debug/`, legacy top-level
/// `raw/` / `wiki/` / `scratch/`, `agents_log/`, and root `PASTE_ME.md`. Does not remove the
/// workdir root itself.
pub fn clean_horde_workdir(spec: &HordeSpec) -> Result<(), Box<dyn std::error::Error>> {
    for rel in ["debug", "raw", "wiki", "scratch", "agents_log"] {
        let p = spec.workdir.join(rel);
        if p.exists() {
            let _ = if p.is_dir() {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p)
            };
        }
    }
    let _ = std::fs::remove_file(spec.workdir.join("PASTE_ME.md"));
    Ok(())
}

pub fn prepare_workdir_on_startup_with_policy(
    spec: &HordeSpec,
    clean_on_startup: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&spec.workdir)?;
    if !clean_on_startup {
        return Ok(());
    }
    clean_horde_workdir(spec)?;
    Ok(())
}

/// Discover all horde directories under `roots` (each root must contain a `horde.md`).
pub fn discover_hordes(roots: &[PathBuf]) -> Vec<HordeSpec> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for r in roots {
        if !r.exists() {
            continue;
        }
        let direct = r.join("horde.md");
        if direct.exists() {
            if let Ok(spec) = load_horde(r)
                && seen.insert(spec.id.clone())
            {
                out.push(spec);
            }
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(r) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && p.join("horde.md").exists() {
                    match load_horde(&p) {
                        Ok(spec) => {
                            if seen.insert(spec.id.clone()) {
                                out.push(spec);
                            }
                        }
                        Err(err) => log::warn!("horde load failed at {}: {}", p.display(), err),
                    }
                }
            }
        }
    }
    out
}

/// API wire vocabulary for run statuses. The store speaks the canonical enum
/// names (`done` / `error`); `/api/*` responses and the UI keep the historical
/// `completed` / `failed` strings — this function is the single owner of that mapping.
pub fn api_run_status(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Done => "completed",
        RunStatus::Error => "failed",
        other => other.as_str(),
    }
}

/// API wire vocabulary for step statuses (`success` instead of the store's
/// `succeeded`). Also the readiness vocabulary consumed by the horde graph
/// (`next_ready_step_conditional` matches on `pending` / `success`).
pub fn api_step_status(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Succeeded => "success",
        other => other.as_str(),
    }
}

fn serialize_run_status<S: serde::Serializer>(s: &RunStatus, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(api_run_status(*s))
}

fn serialize_step_status<S: serde::Serializer>(s: &StepStatus, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(api_step_status(*s))
}

#[derive(Debug, Clone, Serialize)]
pub struct RunStepRecord {
    pub step: String,
    pub agent_id: String,
    pub task_id: String,
    #[serde(serialize_with = "serialize_step_status")]
    pub status: StepStatus,
    /// 1-based execution attempt; loop-backs re-run the step as a new attempt.
    pub attempt: u32,
    pub artifact: Option<String>,
    pub summary: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// `pass` / `fail` from verify-style artifact frontmatter (when applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub run_id: String,
    pub horde_id: String,
    pub prompt: String,
    pub source: Option<String>,
    pub question: String,
    #[serde(serialize_with = "serialize_run_status")]
    pub status: RunStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub current_step_index: usize,
    pub steps: Vec<RunStepRecord>,
    pub events: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub loop_counts: BTreeMap<String, u32>,
    /// How the run was started (`operator` for UI/API operators; non-operator
    /// origins such as `trigger` are auto-resumed on server startup).
    pub origin: String,
    /// Resume attempts spent so far (capped; see [`DEFAULT_RESUME_MAX_ATTEMPTS`]).
    pub resume_count: u32,
    /// True when the run is incomplete in the store but no live orchestrator
    /// task owns it (interrupted by a restart, or awaiting operator input).
    pub resumable: bool,
}

impl RunRecord {
    /// Rebuild an API-shaped record from a store row. Steps are ordered by the
    /// pipeline captured in the run's manifest snapshot (store order is by
    /// `started_at`, which interleaves pending rows).
    pub fn from_persisted(p: PersistedRun) -> Self {
        let pipeline: Vec<String> = p
            .manifest_snapshot
            .as_ref()
            .and_then(|m| m.get("pipeline"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let mut steps: Vec<RunStepRecord> = p
            .steps
            .iter()
            .map(|s| RunStepRecord {
                step: s.step.clone(),
                agent_id: s.agent_id.clone(),
                task_id: s.task_id.clone(),
                status: s.status,
                attempt: s.attempt.max(1) as u32,
                artifact: s.artifact.clone(),
                summary: s.summary.clone(),
                started_at: s.started_at.clone().unwrap_or_else(|| p.started_at.clone()),
                finished_at: s.finished_at.clone(),
                outcome: s.outcome.clone(),
            })
            .collect();
        if !pipeline.is_empty() {
            steps.sort_by_key(|s| {
                pipeline
                    .iter()
                    .position(|name| name == &s.step)
                    .unwrap_or(usize::MAX)
            });
        }
        let current_step_index = p
            .current_step
            .as_deref()
            .and_then(|c| steps.iter().position(|s| s.step == c))
            .unwrap_or(0);
        Self {
            run_id: p.run_id,
            horde_id: p.horde_id,
            prompt: p.prompt,
            source: p.source,
            question: p.question,
            status: p.status,
            started_at: p.started_at,
            finished_at: p.finished_at,
            current_step_index,
            steps,
            events: p.events,
            loop_counts: p.loop_counts,
            origin: p.origin,
            resume_count: p.resume_count.max(0) as u32,
            resumable: false,
        }
    }
}

/// Active runs by run_id, used both to advance the pipeline on TaskFinished and
/// to render run snapshots in the UI.
#[derive(Default)]
pub struct RunRegistry {
    pub runs: HashMap<String, RunRecord>,
}

pub type SharedRunRegistry = Arc<Mutex<RunRegistry>>;

#[derive(Clone)]
pub struct HordeManager {
    pub specs: Arc<Vec<HordeSpec>>,
    pub runs: SharedRunRegistry,
    pub broker: Arc<MpscBroker>,
    pub federation: Arc<FederationOrchestrator>,
    pub orchestrator_id: String,
    /// System of record for runs: every state transition writes through to the
    /// store; the in-memory registry is a cache for the active-run hot path.
    pub store: RunStore,
    /// Cap on resume attempts per run before it is errored out
    /// ([`DEFAULT_RESUME_MAX_ATTEMPTS`]; `[horde] resume_max_attempts` in config).
    pub resume_max_attempts: u32,
}

impl HordeManager {
    pub fn new(
        specs: Vec<HordeSpec>,
        broker: Arc<MpscBroker>,
        federation: Arc<FederationOrchestrator>,
        store: RunStore,
    ) -> Self {
        Self {
            specs: Arc::new(specs),
            runs: Arc::new(Mutex::new(RunRegistry::default())),
            broker,
            federation,
            orchestrator_id: federation_orchestrator_id(),
            store,
            resume_max_attempts: DEFAULT_RESUME_MAX_ATTEMPTS,
        }
    }

    async fn persist_step(&self, run_id: &str, step: &RunStepRecord) {
        let update = StepUpdate {
            step: step.step.clone(),
            agent_id: step.agent_id.clone(),
            task_id: step.task_id.clone(),
            status: step.status,
            attempt: step.attempt as i64,
            outcome: step.outcome.clone(),
            artifact: step.artifact.clone(),
            summary: step.summary.clone(),
        };
        if let Err(e) = self.store.upsert_step(run_id, &update).await {
            log::warn!("run store: step write failed run={run_id} step={}: {e}", step.step);
        }
    }

    async fn persist_run_status(&self, run_id: &str, status: RunStatus, result: Option<&str>) {
        if let Err(e) = self.store.update_run_status(run_id, status, result).await {
            log::warn!("run store: status write failed run={run_id}: {e}");
        }
    }

    async fn persist_current_step(&self, run_id: &str, step: Option<&str>) {
        if let Err(e) = self.store.set_current_step(run_id, step).await {
            log::warn!("run store: current-step write failed run={run_id}: {e}");
        }
    }

    async fn persist_loop_counts(&self, run_id: &str, loop_counts: &BTreeMap<String, u32>) {
        if let Err(e) = self.store.set_loop_counts(run_id, loop_counts).await {
            log::warn!("run store: loop-counts write failed run={run_id}: {e}");
        }
    }

    async fn persist_event(&self, run_id: &str, event: &serde_json::Value) {
        if let Err(e) = self.store.record_event(run_id, event).await {
            log::warn!("run store: event write failed run={run_id}: {e}");
        }
    }

    pub fn find(&self, horde_id: &str) -> Option<&HordeSpec> {
        self.specs.iter().find(|s| s.id == horde_id)
    }

    /// Compose the canonical task_id for a (horde, run, step) triple.
    pub fn task_id(&self, horde: &str, run_id: &str, step: &str) -> String {
        format!("{}::{}::{}", horde, run_id, step)
    }

    pub fn parse_task_id(task_id: &str) -> Option<(String, String, String)> {
        let mut parts = task_id.splitn(3, "::");
        let horde = parts.next()?.to_string();
        let run_id = parts.next()?.to_string();
        let step = parts.next()?.to_string();
        if horde.is_empty() || run_id.is_empty() || step.is_empty() {
            return None;
        }
        Some((horde, run_id, step))
    }

    pub async fn publish(&self, env: &AclEnvelope) {
        if let Err(e) = self.broker.publish(env).await {
            log::warn!("horde publish failed: {}", e);
        }
    }

    pub fn build_envelope(&self, topic: &str, message: AclMessage) -> AclEnvelope {
        AclEnvelope::new(topic.to_string(), self.orchestrator_id.clone(), message)
    }

    /// Build the JSON instruction passed in `TaskDelegate.instruction`.
    pub fn build_instruction(
        &self,
        spec: &HordeSpec,
        run: &RunRecord,
        step: &str,
        previous_artifact: Option<&str>,
    ) -> String {
        let kind = spec
            .sub_agent(step)
            .map(|s| s.kind.clone())
            .unwrap_or_else(|| step.to_string());
        let tool_ids = spec
            .sub_agent(step)
            .map(|s| s.tool_ids.clone())
            .unwrap_or_default();
        let project_path = run
            .source
            .as_deref()
            .and_then(kowalski_core::source_bundle::extract_project_path_from_source)
            .map(|p| p.display().to_string());
        let payload = json!({
            "horde": spec.id,
            "run_id": run.run_id,
            "step": step,
            "kind": kind,
            "source": run.source,
            "question": run.question,
            "previous_artifact": previous_artifact,
            "horde_root": spec.root_path.display().to_string(),
            "workdir": spec.workdir.display().to_string(),
            "project_path": project_path,
            "tool_ids": tool_ids,
        });
        payload.to_string()
    }

    fn step_status_map(run: &RunRecord) -> BTreeMap<String, &str> {
        run.steps
            .iter()
            .map(|s| (s.step.clone(), api_step_status(s.status)))
            .collect()
    }

    fn artifact_for_step(run: &RunRecord, step: &str) -> Option<String> {
        run.steps
            .iter()
            .find(|s| s.step == step)
            .and_then(|s| s.artifact.clone())
    }

    fn previous_artifact_for_step(spec: &HordeSpec, run: &RunRecord, step: &str) -> Option<String> {
        single_predecessor(&spec.execution_graph, step)
            .and_then(|pred| Self::artifact_for_step(run, &pred))
    }

    /// Start a new horde run: register, emit RunStarted, delegate first ready step.
    /// `origin` is [`RUN_ORIGIN_OPERATOR`] for UI/API operators; non-operator
    /// origins (e.g. `trigger`) are auto-resumed if a restart interrupts them.
    pub async fn start_run(
        &self,
        horde_id: &str,
        prompt: &str,
        source: Option<&str>,
        question: Option<&str>,
        origin: &str,
    ) -> Result<RunRecord, String> {
        let spec = self
            .find(horde_id)
            .ok_or_else(|| format!("unknown horde id: {}", horde_id))?
            .clone();
        if spec.pipeline.is_empty() {
            return Err(format!("horde {} has empty pipeline", horde_id));
        }
        let run_id = new_run_id();
        let started_at = now_ts();
        let q = question
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| spec.default_question.clone());
        let mut record = RunRecord {
            run_id: run_id.clone(),
            horde_id: spec.id.clone(),
            prompt: prompt.to_string(),
            source: source.map(ToString::to_string),
            question: q.clone(),
            status: RunStatus::Running,
            started_at: started_at.clone(),
            finished_at: None,
            current_step_index: 0,
            steps: spec
                .pipeline
                .iter()
                .map(|s| RunStepRecord {
                    step: s.clone(),
                    agent_id: spec
                        .sub_agent(s)
                        .map(|x| x.default_agent_id.clone())
                        .unwrap_or_default(),
                    task_id: self.task_id(&spec.id, &run_id, s),
                    status: StepStatus::Pending,
                    attempt: 1,
                    artifact: None,
                    summary: None,
                    started_at: started_at.clone(),
                    finished_at: None,
                    outcome: None,
                })
                .collect(),
            events: Vec::new(),
            loop_counts: BTreeMap::new(),
            origin: origin.to_string(),
            resume_count: 0,
            resumable: false,
        };

        self.store
            .create_run(&NewRun {
                run_id: run_id.clone(),
                horde_id: spec.id.clone(),
                prompt: prompt.to_string(),
                source: source.map(ToString::to_string),
                question: q.clone(),
                manifest_snapshot: serde_json::to_value(&spec).ok(),
                origin: origin.to_string(),
            })
            .await
            .map_err(|e| format!("run store: create run failed: {e}"))?;
        for step in &record.steps {
            self.persist_step(&run_id, step).await;
        }
        self.persist_run_status(&run_id, RunStatus::Running, None).await;

        let started_msg = AclMessage::RunStarted {
            run_id: run_id.clone(),
            horde: spec.id.clone(),
            prompt: prompt.to_string(),
            source: source.map(ToString::to_string),
            question: Some(q.clone()),
            pipeline: spec.pipeline.clone(),
        };
        let env = self.build_envelope(&spec.topic, started_msg);
        record.events.push(envelope_summary(&env));
        self.persist_event(&run_id, &envelope_summary(&env)).await;
        self.publish(&env).await;

        {
            let mut runs = self.runs.lock().await;
            runs.runs.insert(run_id.clone(), record.clone());
        }

        let first_step = {
            let status = Self::step_status_map(&record);
            next_ready_step_conditional(
                &spec.pipeline,
                &spec.execution_graph,
                &status,
                &BTreeMap::new(),
            )
            .ok_or_else(|| {
                format!("horde {} has no runnable step after RunStarted", horde_id)
            })?
        };

        if let Err(e) = self
            .delegate_step(&spec, &run_id, &first_step, None)
            .await
        {
            self.fail_run(
                &spec,
                &run_id,
                &format!("delegate first step failed: {}", e),
                Some(&first_step),
            )
            .await;
        }
        let runs = self.runs.lock().await;
        runs.runs
            .get(&run_id)
            .cloned()
            .ok_or_else(|| "run vanished after start".to_string())
    }

    async fn delegate_step(
        &self,
        spec: &HordeSpec,
        run_id: &str,
        step_name: &str,
        previous_artifact: Option<&str>,
    ) -> Result<(), String> {
        let sub = spec
            .sub_agent(step_name)
            .ok_or_else(|| format!("missing sub-agent {} in horde {}", step_name, spec.id))?
            .clone();
        let task_id = self.task_id(&spec.id, run_id, step_name);

        let instruction;
        let assigned_envelope;
        let run_for_log;
        {
            let mut runs = self.runs.lock().await;
            let run = runs
                .runs
                .get_mut(run_id)
                .ok_or_else(|| format!("run {} no longer tracked", run_id))?;
            run.current_step_index = spec
                .pipeline
                .iter()
                .position(|s| s == step_name)
                .unwrap_or(0);
            let mut delegating_step = None;
            if let Some(step) = run.steps.iter_mut().find(|s| s.step == step_name) {
                step.status = StepStatus::Delegating;
                step.task_id = task_id.clone();
                step.agent_id = sub.default_agent_id.clone();
                step.started_at = now_ts();
                delegating_step = Some(step.clone());
            }
            instruction = self.build_instruction(spec, run, step_name, previous_artifact);
            let assigned_msg = AclMessage::TaskAssigned {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                step: step_name.to_string(),
                from: self.orchestrator_id.clone(),
                to: sub.default_agent_id.clone(),
                task_id: task_id.clone(),
                instruction: instruction.clone(),
            };
            assigned_envelope = self.build_envelope(&spec.topic, assigned_msg);
            run.events.push(envelope_summary(&assigned_envelope));
            run_for_log = run.clone();
            self.persist_current_step(run_id, Some(step_name)).await;
            if let Some(step) = delegating_step {
                self.persist_step(run_id, &step).await;
            }
            self.persist_event(run_id, &envelope_summary(&assigned_envelope))
                .await;
        }
        self.publish(&assigned_envelope).await;
        log::info!(
            "horde {} run {} step {} -> capability {}",
            spec.id,
            run_id,
            step_name,
            sub.capability
        );
        let _ = run_for_log;

        match self
            .federation
            .delegate_first_match(&task_id, &instruction, &sub.capability)
            .await
        {
            Ok(Some(_)) => Ok(()),
            Ok(None) => {
                let reason = format!(
                    "no worker registered for capability `{}` (start the {} worker)",
                    sub.capability, sub.display_name
                );
                Err(reason)
            }
            Err(e) => Err(format!("federation delegate error: {}", e)),
        }
    }

    /// Mark a step as finished and advance the pipeline (or finalize the run).
    pub async fn handle_task_finished(
        &self,
        run_id: &str,
        step: &str,
        success: bool,
        artifact: Option<&str>,
        summary: &str,
    ) {
        let spec_opt = {
            let runs = self.runs.lock().await;
            runs.runs
                .get(run_id)
                .and_then(|r| self.find(&r.horde_id).cloned())
        };
        let Some(spec) = spec_opt else {
            log::warn!("horde TaskFinished for unknown run_id={}", run_id);
            return;
        };

        if !success {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                if let Some(step_record) = run.steps.iter_mut().find(|s| s.step == step) {
                    step_record.status = StepStatus::Failed;
                    step_record.summary = Some(summary.to_string());
                    step_record.finished_at = Some(now_ts());
                    step_record.outcome = Some(StageStatus::Fail.as_str().to_string());
                    let failed_step = step_record.clone();
                    self.persist_step(run_id, &failed_step).await;
                }
            }
            drop(runs);
            self.fail_run(
                &spec,
                run_id,
                &format!("step {} failed: {}", step, summary),
                Some(step),
            )
            .await;
            return;
        }

        let (next_step, route_error, route_notice) = {
            let mut runs = self.runs.lock().await;
            let Some(run) = runs.runs.get_mut(run_id) else {
                return;
            };
            if !run.status.is_incomplete() {
                return;
            }
            let already_finalized = run
                .steps
                .iter()
                .find(|s| s.step == step)
                .map(|s| matches!(s.status, StepStatus::Succeeded | StepStatus::Failed))
                .unwrap_or(false);
            if already_finalized {
                return;
            }

            let outcome = Self::step_outcome_for(&spec, step, artifact, true);
            if let Some(step_record) = run.steps.iter_mut().find(|s| s.step == step) {
                step_record.status = StepStatus::Succeeded;
                step_record.artifact = artifact.map(ToString::to_string);
                step_record.summary = Some(summary.to_string());
                step_record.finished_at = Some(now_ts());
                step_record.outcome = Some(outcome.as_str().to_string());
                let finished_step = step_record.clone();
                self.persist_step(run_id, &finished_step).await;
            }
            let finished_event = json!({
                "kind": "task_finished",
                "step": step,
                "success": true,
                "outcome": outcome.as_str(),
                "artifact": artifact,
                "summary": summary,
                "ts": now_ts(),
            });
            self.persist_event(run_id, &finished_event).await;
            run.events.push(finished_event);

            let mut route_notice = None;
            let (next_step, route_error) = if has_conditional_outbound(&spec.execution_graph.edges, step) {
                match select_next_from_outcome(
                    &spec.pipeline,
                    &spec.execution_graph.edges,
                    step,
                    outcome,
                    &run.loop_counts,
                ) {
                    Some(next) => {
                        let is_back = is_loop_back_step(&spec.pipeline, step, &next);
                        let loop_count = if is_back {
                            let key = loop_edge_key(step, &next);
                            *run.loop_counts.entry(key.clone()).or_insert(0) += 1;
                            let mut reset_steps = Vec::new();
                            for s in retry_span(&spec.pipeline, &next, step) {
                                if let Some(rec) = run.steps.iter_mut().find(|r| r.step == s) {
                                    rec.status = StepStatus::Pending;
                                    rec.attempt += 1;
                                    rec.artifact = None;
                                    rec.summary = None;
                                    rec.finished_at = None;
                                    rec.outcome = None;
                                    reset_steps.push(rec.clone());
                                }
                            }
                            self.persist_loop_counts(run_id, &run.loop_counts).await;
                            for rec in &reset_steps {
                                self.persist_step(run_id, rec).await;
                            }
                            run.loop_counts.get(&key).copied()
                        } else {
                            None
                        };
                        let verify_excerpt = Self::verify_excerpt_for_step(
                            &spec,
                            step,
                            artifact,
                        );
                        route_notice = Some((next.clone(), outcome, is_back, loop_count, verify_excerpt));
                        (Some(next), None)
                    }
                    None => (
                        None,
                        Some(format!(
                            "no route for step `{step}` outcome `{}` (check `when` / `max_loops`)",
                            outcome.as_str()
                        )),
                    ),
                }
            } else if all_steps_successful(&spec.pipeline, &Self::step_status_map(run)) {
                (None, None)
            } else {
                let outcomes = Self::step_outcome_map(&spec, run);
                (
                    next_ready_step_conditional(
                        &spec.pipeline,
                        &spec.execution_graph,
                        &Self::step_status_map(run),
                        &outcomes,
                    ),
                    None,
                )
            };
            (next_step, route_error, route_notice)
        };

        if let Some((next, outcome, is_back, loop_count, verify_excerpt)) = route_notice {
            self.publish_step_routed(
                &spec,
                run_id,
                step,
                outcome,
                &next,
                is_back,
                loop_count,
                verify_excerpt.as_deref(),
            )
            .await;
        }

        if let Some(err) = route_error {
            self.fail_run(&spec, run_id, &err, Some(step)).await;
            return;
        }

        if let Some(next) = next_step {
            let prev = {
                let runs = self.runs.lock().await;
                runs.runs
                    .get(run_id)
                    .and_then(|run| Self::previous_artifact_for_step(&spec, run, &next))
            };
            if let Err(e) = self
                .delegate_step(&spec, run_id, &next, prev.as_deref())
                .await
            {
                self.fail_run(&spec, run_id, &e, Some(&next)).await;
            }
        } else {
            let all_done = {
                let runs = self.runs.lock().await;
                runs.runs
                    .get(run_id)
                    .map(|run| all_steps_successful(&spec.pipeline, &Self::step_status_map(run)))
                    .unwrap_or(false)
            };
            if all_done {
                self.complete_run(&spec, run_id).await;
            } else {
                self.fail_run(
                    &spec,
                    run_id,
                    "pipeline stalled: no step ready and run incomplete",
                    None,
                )
                .await;
            }
        }
    }

    fn read_artifact_text(spec: &HordeSpec, artifact: &str) -> Option<String> {
        let p = PathBuf::from(artifact);
        if p.is_file() {
            return std::fs::read_to_string(p).ok();
        }
        std::fs::read_to_string(spec.workdir.join(artifact)).ok()
    }

    fn verify_excerpt_for_step(
        spec: &HordeSpec,
        step: &str,
        artifact: Option<&str>,
    ) -> Option<String> {
        let kind = spec.sub_agent(step).map(|s| s.kind.as_str()).unwrap_or("");
        if kind != "verify" {
            return None;
        }
        let body = artifact.and_then(|a| Self::read_artifact_text(spec, a))?;
        Some(verify_output_excerpt(&body, 1_200))
    }

    async fn publish_step_routed(
        &self,
        spec: &HordeSpec,
        run_id: &str,
        from_step: &str,
        outcome: StageStatus,
        next_step: &str,
        is_loop_back: bool,
        loop_count: Option<u32>,
        verify_excerpt: Option<&str>,
    ) {
        let msg = AclMessage::StepRouted {
            run_id: run_id.to_string(),
            horde: spec.id.clone(),
            from_step: from_step.to_string(),
            outcome: outcome.as_str().to_string(),
            next_step: next_step.to_string(),
            is_loop_back,
            loop_count,
            verify_excerpt: verify_excerpt.map(ToString::to_string),
        };
        let env = self.build_envelope(&spec.topic, msg);
        {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                run.events.push(envelope_summary(&env));
                self.persist_event(run_id, &envelope_summary(&env)).await;
            }
        }
        self.publish(&env).await;
    }

    fn step_outcome_for(
        spec: &HordeSpec,
        step: &str,
        artifact: Option<&str>,
        worker_success: bool,
    ) -> StageStatus {
        if !worker_success {
            return StageStatus::Fail;
        }
        let kind = spec
            .sub_agent(step)
            .map(|s| s.kind.as_str())
            .unwrap_or("");
        if matches!(kind, "verify" | "apply") {
            if let Some(path) = artifact {
                if let Some(body) = Self::read_artifact_text(spec, path) {
                    return parse_stage_status_from_artifact(&body).unwrap_or(StageStatus::Fail);
                }
            }
            return StageStatus::Fail;
        }
        StageStatus::Pass
    }

    fn step_outcome_map(spec: &HordeSpec, run: &RunRecord) -> BTreeMap<String, StageStatus> {
        let mut map = BTreeMap::new();
        for s in &run.steps {
            if s.status != StepStatus::Succeeded {
                continue;
            }
            if let Some(ref o) = s.outcome {
                if let Some(parsed) = StageStatus::parse(o) {
                    map.insert(s.step.clone(), parsed);
                    continue;
                }
            }
            map.insert(
                s.step.clone(),
                Self::step_outcome_for(spec, &s.step, s.artifact.as_deref(), true),
            );
        }
        map
    }

    async fn complete_run(&self, spec: &HordeSpec, run_id: &str) {
        let artifacts: Vec<(String, String)> = {
            let mut runs = self.runs.lock().await;
            let Some(run) = runs.runs.get_mut(run_id) else {
                return;
            };
            run.status = RunStatus::Done;
            run.finished_at = Some(now_ts());
            self.persist_run_status(run_id, RunStatus::Done, None).await;
            run.steps
                .iter()
                .filter_map(|s| s.artifact.clone().map(|a| (s.step.clone(), a)))
                .collect()
        };
        let paste_path = spec.workdir.join("PASTE_ME.md");
        let handoff_markdown = std::fs::read_to_string(&paste_path).ok().map(|s| {
            const MAX: usize = 48_000;
            if s.len() <= MAX {
                s
            } else {
                format!(
                    "{}\n\n_(truncated to {} chars for federation payload)_\n",
                    s.chars().take(MAX).collect::<String>(),
                    MAX
                )
            }
        });
        let env = self.build_envelope(
            &spec.topic,
            AclMessage::RunFinished {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                artifacts: artifacts.clone(),
                text: Some(format!(
                    "{} run completed; {} artifact(s). Markdown hand-off: `handoff_markdown` in this event; file `{}`.",
                    spec.display_name,
                    artifacts.len(),
                    paste_path.display()
                )),
                handoff_markdown,
            },
        );
        {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                run.events.push(envelope_summary(&env));
                self.persist_event(run_id, &envelope_summary(&env)).await;
            }
        }
        self.publish(&env).await;
    }

    async fn fail_run(&self, spec: &HordeSpec, run_id: &str, reason: &str, step: Option<&str>) {
        {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                run.status = RunStatus::Error;
                run.finished_at = Some(now_ts());
                let failed_event = json!({
                    "kind": "run_failed",
                    "reason": reason,
                    "step": step,
                    "ts": now_ts(),
                });
                self.persist_run_status(run_id, RunStatus::Error, Some(reason))
                    .await;
                self.persist_event(run_id, &failed_event).await;
                run.events.push(failed_event);
            }
        }
        let env = self.build_envelope(
            &spec.topic,
            AclMessage::RunFailed {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                reason: reason.to_string(),
                step: step.map(ToString::to_string),
            },
        );
        self.publish(&env).await;
    }

    /// Append an inter-agent or progress event to the run history (best-effort).
    pub async fn record_event(&self, run_id: &str, event: &AclMessage) {
        let mut runs = self.runs.lock().await;
        if let Some(run) = runs.runs.get_mut(run_id) {
            let value = serde_json::to_value(event).unwrap_or(json!({}));
            self.persist_event(run_id, &value).await;
            run.events.push(value);
        }
    }

    /// One run from the store (survives restarts), API-shaped.
    pub async fn persisted_run(&self, run_id: &str) -> Result<Option<RunRecord>, String> {
        let run = self
            .store
            .get_run(run_id)
            .await
            .map_err(|e| format!("run store: {e}"))?;
        let mut record = run.map(RunRecord::from_persisted);
        if let Some(r) = record.as_mut() {
            r.resumable = self.is_resumable(r).await;
        }
        Ok(record)
    }

    /// One page of a horde's runs from the store, newest first, steps included.
    pub async fn persisted_runs(
        &self,
        horde_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<RunRecord>, String> {
        let page = self
            .store
            .list_runs(Some(horde_id), limit, offset)
            .await
            .map_err(|e| format!("run store: {e}"))?;
        let mut out = Vec::with_capacity(page.len());
        for run in page {
            let full = self
                .store
                .get_run(&run.run_id)
                .await
                .map_err(|e| format!("run store: {e}"))?
                .unwrap_or(run);
            let mut record = RunRecord::from_persisted(full);
            record.resumable = self.is_resumable(&record).await;
            out.push(record);
        }
        Ok(out)
    }

    /// A run is resumable when the store says it never reached a terminal state
    /// and no orchestrator task in this process owns it (interrupted by a
    /// restart, or parked awaiting operator input).
    async fn is_resumable(&self, record: &RunRecord) -> bool {
        if !record.status.is_incomplete() {
            return false;
        }
        let runs = self.runs.lock().await;
        !runs.runs.contains_key(&record.run_id)
    }

    /// Execution graph for resume-point computation, rebuilt from the manifest
    /// snapshot captured at run start (falls back to the live spec when the
    /// snapshot is missing or unparseable).
    fn snapshot_execution(
        spec: &HordeSpec,
        snapshot: Option<&serde_json::Value>,
    ) -> (Vec<String>, ExecutionGraph) {
        if let Some(snap) = snapshot
            && let Some(pipeline) = snap
                .get("pipeline")
                .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            && !pipeline.is_empty()
        {
            let edges: Vec<HordeEdge> = snap
                .get("manifest_edges")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let edge_slice = (!edges.is_empty()).then_some(edges.as_slice());
            if let Ok(graph) = resolve_execution_graph(&pipeline, edge_slice) {
                return (pipeline, graph);
            }
        }
        (spec.pipeline.clone(), spec.execution_graph.clone())
    }

    /// Resume one interrupted run: mark the step that was in flight when the
    /// server died as a failed attempt, recompute the next ready step from the
    /// snapshot graph + persisted step outcomes (loop counts intact), and
    /// re-delegate it. Artifacts of completed steps are never regenerated.
    /// A failed resume leaves the run resumable until the attempt cap is spent,
    /// then the run goes to `error` with a reason.
    pub async fn resume_run(&self, run_id: &str) -> Result<RunRecord, String> {
        let persisted = self
            .store
            .get_run(run_id)
            .await
            .map_err(|e| format!("run store: {e}"))?
            .ok_or_else(|| format!("run {run_id} not found"))?;
        if !persisted.status.is_incomplete() {
            return Err(format!(
                "run {run_id} is {}, not resumable",
                api_run_status(persisted.status)
            ));
        }
        {
            let runs = self.runs.lock().await;
            if runs.runs.contains_key(run_id) {
                return Err(format!("run {run_id} is already active"));
            }
        }
        let spec = self
            .find(&persisted.horde_id)
            .ok_or_else(|| {
                format!(
                    "horde {} for run {run_id} is no longer in the catalog",
                    persisted.horde_id
                )
            })?
            .clone();

        if persisted.resume_count >= self.resume_max_attempts as i64 {
            let reason = format!(
                "resume attempts exhausted ({} of {})",
                persisted.resume_count, self.resume_max_attempts
            );
            self.mark_unresumable(&spec, run_id, &reason).await;
            return Err(reason);
        }
        let attempt_no = self
            .store
            .increment_resume_count(run_id)
            .await
            .map_err(|e| format!("run store: {e}"))?;

        let (pipeline, graph) = Self::snapshot_execution(&spec, persisted.manifest_snapshot.as_ref());
        let mut record = RunRecord::from_persisted(persisted);
        record.status = RunStatus::Running;
        record.finished_at = None;
        record.resume_count = attempt_no.max(0) as u32;

        // The interrupted in-flight step counts as a failed attempt: back to
        // pending with attempt+1, so the readiness scan below re-delegates it.
        let mut reset_steps = Vec::new();
        for s in record.steps.iter_mut() {
            if matches!(s.status, StepStatus::Delegating | StepStatus::Running) {
                s.status = StepStatus::Pending;
                s.attempt += 1;
                s.artifact = None;
                s.summary = None;
                s.finished_at = None;
                s.outcome = None;
                reset_steps.push(s.clone());
            }
        }
        for s in &reset_steps {
            self.persist_step(run_id, s).await;
        }
        self.persist_run_status(run_id, RunStatus::Running, None).await;

        let next = {
            let status = Self::step_status_map(&record);
            let outcomes = Self::step_outcome_map(&spec, &record);
            next_ready_step_conditional(&pipeline, &graph, &status, &outcomes)
        };

        {
            let mut runs = self.runs.lock().await;
            runs.runs.insert(run_id.to_string(), record.clone());
        }

        let Some(next) = next else {
            if all_steps_successful(&pipeline, &Self::step_status_map(&record)) {
                // The crash landed between the last step finishing and run completion.
                self.complete_run(&spec, run_id).await;
                return self
                    .persisted_run(run_id)
                    .await?
                    .ok_or_else(|| "run vanished after resume".to_string());
            }
            let reason =
                "resume found no runnable step (check `when` / `max_loops`)".to_string();
            self.resume_attempt_failed(&spec, run_id, attempt_no, &reason).await;
            return Err(reason);
        };

        let resumed_event = json!({
            "kind": "run_resumed",
            "step": next,
            "resume_attempt": attempt_no,
            "ts": now_ts(),
        });
        self.persist_event(run_id, &resumed_event).await;
        {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                run.events.push(resumed_event);
            }
        }
        // Feed marker; recorded once via the orchestrator loop's own subscription.
        let marker = self.build_envelope(
            &spec.topic,
            AclMessage::AgentMessage {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                from: self.orchestrator_id.clone(),
                step: None,
                text: format!(
                    "run resumed (attempt {attempt_no}): continuing from step `{next}`"
                ),
            },
        );
        self.publish(&marker).await;

        let prev = {
            let runs = self.runs.lock().await;
            runs.runs
                .get(run_id)
                .and_then(|run| Self::previous_artifact_for_step(&spec, run, &next))
        };
        if let Err(e) = self.delegate_step(&spec, run_id, &next, prev.as_deref()).await {
            self.resume_attempt_failed(&spec, run_id, attempt_no, &e).await;
            return Err(e);
        }
        let runs = self.runs.lock().await;
        runs.runs
            .get(run_id)
            .cloned()
            .ok_or_else(|| "run vanished after resume".to_string())
    }

    /// A resume attempt failed before the run got moving again. Below the cap
    /// the run leaves the active registry and stays resumable; at the cap it
    /// goes to `error` with the reason.
    async fn resume_attempt_failed(
        &self,
        spec: &HordeSpec,
        run_id: &str,
        attempts: i64,
        reason: &str,
    ) {
        let failed_event = json!({
            "kind": "resume_failed",
            "reason": reason,
            "resume_attempt": attempts,
            "ts": now_ts(),
        });
        self.persist_event(run_id, &failed_event).await;
        if attempts >= self.resume_max_attempts as i64 {
            self.fail_run(
                spec,
                run_id,
                &format!("resume failed {attempts} time(s), giving up: {reason}"),
                None,
            )
            .await;
            return;
        }
        let mut runs = self.runs.lock().await;
        runs.runs.remove(run_id);
    }

    /// Error out a run that can no longer be resumed (attempt cap exhausted).
    /// Unlike [`Self::fail_run`] this does not require a registry entry.
    async fn mark_unresumable(&self, spec: &HordeSpec, run_id: &str, reason: &str) {
        self.persist_run_status(run_id, RunStatus::Error, Some(reason)).await;
        let failed_event = json!({
            "kind": "run_failed",
            "reason": reason,
            "ts": now_ts(),
        });
        self.persist_event(run_id, &failed_event).await;
        let env = self.build_envelope(
            &spec.topic,
            AclMessage::RunFailed {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                reason: reason.to_string(),
                step: None,
            },
        );
        self.publish(&env).await;
    }

    /// Startup reconciliation of interrupted runs. Classifies every incomplete
    /// run in the store: `awaiting_input` runs are durable by construction and
    /// stay resumable on demand; interrupted operator runs are surfaced as
    /// resumable; non-operator runs (e.g. future trigger-fired ones) are
    /// auto-resumed.
    pub async fn resume_scan(&self) {
        let incomplete = match self.store.incomplete_runs().await {
            Ok(runs) => runs,
            Err(e) => {
                log::warn!("resume scan: incomplete_runs failed: {e}");
                return;
            }
        };
        for run in incomplete {
            let run_id = run.run_id.clone();
            {
                let runs = self.runs.lock().await;
                if runs.runs.contains_key(&run_id) {
                    continue;
                }
            }
            if run.status == RunStatus::AwaitingInput {
                log::info!("resume scan: run {run_id} awaiting input — resumable on demand");
                continue;
            }
            let interrupted_event = json!({ "kind": "run_interrupted", "ts": now_ts() });
            self.persist_event(&run_id, &interrupted_event).await;
            if run.origin == RUN_ORIGIN_OPERATOR {
                log::info!(
                    "resume scan: run {run_id} (horde {}) interrupted — resume via POST /api/hordes/{}/runs/{run_id}/resume",
                    run.horde_id,
                    run.horde_id
                );
                continue;
            }
            match self.resume_run(&run_id).await {
                Ok(_) => log::info!(
                    "resume scan: auto-resumed run {run_id} (origin {})",
                    run.origin
                ),
                Err(e) => {
                    log::warn!("resume scan: auto-resume of run {run_id} failed: {e}")
                }
            }
        }
    }
}

fn envelope_summary(env: &AclEnvelope) -> serde_json::Value {
    serde_json::to_value(&env.payload).unwrap_or(json!({}))
}

pub fn federation_orchestrator_id() -> String {
    "horde-orchestrator".to_string()
}

/// Spawn the broker subscription loop that drives horde runs forward when a
/// sub-agent worker reports `TaskFinished`. Spawns one task per distinct horde topic.
pub fn spawn_orchestrator_loop(manager: HordeManager) {
    let mut topics: Vec<String> = manager.specs.iter().map(|s| s.topic.clone()).collect();
    topics.sort();
    topics.dedup();
    if topics.is_empty() {
        topics.push(DEFAULT_TOPIC.to_string());
    }
    for topic in topics {
        let m = manager.clone();
        tokio::spawn(async move {
            let mut rx = m.broker.subscribe(&topic, 128);
            log::info!("horde orchestrator listening on topic `{}`", topic);
            while let Some(env) = rx.recv().await {
                handle_envelope(&m, env).await;
            }
            log::warn!("horde orchestrator topic `{}` channel closed", topic);
        });
    }
}

async fn handle_envelope(manager: &HordeManager, env: AclEnvelope) {
    match &env.payload {
        AclMessage::TaskFinished {
            run_id,
            horde: _,
            step,
            agent: _,
            success,
            artifact,
            summary,
            ..
        } => {
            manager
                .handle_task_finished(run_id, step, *success, artifact.as_deref(), summary)
                .await;
        }
        AclMessage::AgentMessage { run_id, .. }
        | AclMessage::TaskStarted { run_id, .. }
        | AclMessage::TaskAssigned { run_id, .. }
        | AclMessage::StepRouted { run_id, .. } => {
            manager.record_event(run_id, &env.payload).await;
        }
        AclMessage::TaskResult {
            task_id,
            from_agent,
            outcome,
            success,
        } => {
            // Backward-compat path: legacy `kc.run` workers report only TaskResult. Synthesize
            // a TaskFinished if the task_id matches the canonical horde encoding.
            if let Some((_horde, run_id, step)) = HordeManager::parse_task_id(task_id) {
                let artifact = parse_outcome_artifact(outcome);
                manager
                    .handle_task_finished(&run_id, &step, *success, artifact.as_deref(), outcome)
                    .await;
            } else {
                log::debug!(
                    "TaskResult outside horde encoding ignored (task_id={}, from={})",
                    task_id,
                    from_agent
                );
            }
        }
        _ => {}
    }
}

fn parse_outcome_artifact(outcome: &str) -> Option<String> {
    // Worker emits "run complete; summary=...; report=...; lint=...; log=..."
    // For chaining we prefer the summary path (consumed by next step).
    for token in outcome.split(';').map(str::trim) {
        if let Some(rest) = token.strip_prefix("artifact=") {
            return Some(rest.to_string());
        }
        if let Some(rest) = token.strip_prefix("summary=") {
            return Some(rest.to_string());
        }
    }
    None
}

/// Resolve discovery roots for hordes from environment + repo defaults.
pub fn default_horde_roots(config_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(env) = std::env::var("KOWALSKI_HORDES_DIR") {
        for piece in env.split(':') {
            if !piece.trim().is_empty() {
                roots.push(PathBuf::from(piece.trim()));
            }
        }
    }
    if let Some(c) = config_dir {
        roots.push(c.join("hordes"));
        if let Some(parent) = c.parent() {
            roots.push(parent.join("examples"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("examples"));
    }
    roots.push(PathBuf::from("/opt/ml/kowalski/examples"));
    let mut seen = std::collections::HashSet::new();
    roots.retain(|p| seen.insert(p.clone()));
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use kowalski_core::federation::{AgentRecord, AgentRegistry};

    fn sub(name: &str) -> SubAgentSpec {
        SubAgentSpec {
            name: name.into(),
            kind: "process".into(),
            capability: format!("test.{name}"),
            default_agent_id: format!("agent-{name}"),
            display_name: name.into(),
            description: String::new(),
            prompt_file: None,
            output: None,
            inputs: Vec::new(),
            avatar: None,
            tool_ids: Vec::new(),
        }
    }

    fn test_spec(dir: &Path) -> HordeSpec {
        let pipeline = vec!["a".to_string(), "b".to_string()];
        let execution_graph = resolve_execution_graph(&pipeline, None).unwrap();
        HordeSpec {
            id: "test-horde".into(),
            display_name: "Test Horde".into(),
            description: String::new(),
            capability_prefix: "test".into(),
            pipeline,
            manifest_edges: Vec::new(),
            execution_graph,
            default_question: "default question".into(),
            topic: "test.topic".into(),
            artifacts_root: dir.join("artifacts"),
            workdir: dir.join("work"),
            config_on_startup: false,
            delivery_title: String::new(),
            delivery_note: String::new(),
            delivery_root_rel: String::new(),
            delivery_summary_note: String::new(),
            prompt_tip: String::new(),
            root_path: dir.to_path_buf(),
            sub_agents: vec![sub("a"), sub("b")],
            followup_artifact_dir: dir.join("follow"),
            worker_log_dir: dir.join("logs"),
            run_form: None,
        }
    }

    async fn test_manager(dir: &Path, with_worker: bool) -> HordeManager {
        let broker = Arc::new(MpscBroker::new());
        let registry = Arc::new(AgentRegistry::new());
        if with_worker {
            registry
                .register(AgentRecord {
                    id: "w1".into(),
                    capabilities: vec!["test.a".into(), "test.b".into()],
                })
                .unwrap();
        }
        let federation = Arc::new(FederationOrchestrator::new(registry, broker.clone()));
        let store = RunStore::open("sqlite::memory:").await.unwrap();
        HordeManager::new(vec![test_spec(dir)], broker, federation, store)
    }

    fn step<'a>(run: &'a PersistedRun, name: &str) -> &'a kowalski_core::db::run_store::PersistedRunStep {
        run.steps
            .iter()
            .find(|s| s.step == name)
            .unwrap_or_else(|| panic!("step {name} missing from store"))
    }

    #[tokio::test]
    async fn start_run_writes_run_and_step_rows() {
        let dir = tempfile::tempdir().unwrap();
        let manager = test_manager(dir.path(), true).await;

        let record = manager
            .start_run("test-horde", "do the thing", None, Some("q?"), RUN_ORIGIN_OPERATOR)
            .await
            .unwrap();
        assert_eq!(record.status, RunStatus::Running);

        let persisted = manager.store.get_run(&record.run_id).await.unwrap().unwrap();
        assert_eq!(persisted.status, RunStatus::Running);
        assert_eq!(persisted.question, "q?");
        assert_eq!(persisted.current_step.as_deref(), Some("a"));
        let snapshot = persisted
            .manifest_snapshot
            .as_ref()
            .expect("manifest snapshot stored");
        assert_eq!(snapshot["pipeline"], serde_json::json!(["a", "b"]));
        assert_eq!(step(&persisted, "a").status, StepStatus::Delegating);
        assert_eq!(step(&persisted, "b").status, StepStatus::Pending);
    }

    #[tokio::test]
    async fn task_finished_walk_writes_every_transition() {
        let dir = tempfile::tempdir().unwrap();
        let manager = test_manager(dir.path(), true).await;
        let record = manager
            .start_run("test-horde", "walk", None, None, RUN_ORIGIN_OPERATOR)
            .await
            .unwrap();
        let run_id = record.run_id;

        manager
            .handle_task_finished(&run_id, "a", true, Some("out/a.md"), "a done")
            .await;
        let mid = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(mid.status, RunStatus::Running, "mid-run row stays running");
        assert_eq!(step(&mid, "a").status, StepStatus::Succeeded);
        assert_eq!(step(&mid, "a").artifact.as_deref(), Some("out/a.md"));
        assert_eq!(step(&mid, "b").status, StepStatus::Delegating);
        assert_eq!(mid.current_step.as_deref(), Some("b"));

        manager
            .handle_task_finished(&run_id, "b", true, None, "b done")
            .await;
        let done = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(done.status, RunStatus::Done);
        assert!(done.finished_at.is_some());
        assert_eq!(step(&done, "b").status, StepStatus::Succeeded);
        assert!(step(&done, "b").finished_at.is_some());
        assert!(
            done.events
                .iter()
                .any(|e| e.get("kind").and_then(|k| k.as_str()) == Some("task_finished")),
            "task_finished events recorded in store"
        );

        let api = RunRecord::from_persisted(done);
        assert_eq!(api.steps.iter().map(|s| s.step.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(serde_json::to_value(&api).unwrap()["status"], "completed");
    }

    #[tokio::test]
    async fn failed_step_marks_run_error_in_store() {
        let dir = tempfile::tempdir().unwrap();
        let manager = test_manager(dir.path(), true).await;
        let record = manager
            .start_run("test-horde", "fail walk", None, None, RUN_ORIGIN_OPERATOR)
            .await
            .unwrap();
        let run_id = record.run_id;

        manager
            .handle_task_finished(&run_id, "a", false, None, "worker exploded")
            .await;
        let persisted = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(persisted.status, RunStatus::Error);
        assert!(persisted.finished_at.is_some());
        assert!(persisted.result.as_deref().unwrap_or("").contains("worker exploded"));
        assert_eq!(step(&persisted, "a").status, StepStatus::Failed);
        assert_eq!(
            serde_json::to_value(RunRecord::from_persisted(persisted)).unwrap()["status"],
            "failed"
        );
    }

    #[tokio::test]
    async fn start_run_without_worker_persists_error() {
        let dir = tempfile::tempdir().unwrap();
        let manager = test_manager(dir.path(), false).await;
        let record = manager
            .start_run("test-horde", "no workers", None, None, RUN_ORIGIN_OPERATOR)
            .await
            .unwrap();
        let persisted = manager.store.get_run(&record.run_id).await.unwrap().unwrap();
        assert_eq!(persisted.status, RunStatus::Error);
        assert!(
            persisted
                .result
                .as_deref()
                .unwrap_or("")
                .contains("no worker registered")
        );
    }

    fn write_fixture_horde(root: &Path) {
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::write(
            root.join("horde.md"),
            "---\nid = \"it-horde\"\ndisplay_name = \"IT Horde\"\ndescription = \"integration fixture\"\npipeline = [\"a\", \"b\"]\n---\n# IT Horde\n",
        )
        .unwrap();
        for (name, cap) in [("a", "test.a"), ("b", "test.b")] {
            std::fs::write(
                root.join("agents").join(format!("{name}.md")),
                format!("---\nname = \"{name}\"\nkind = \"process\"\ncapability = \"{cap}\"\n---\n"),
            )
            .unwrap();
        }
    }

    async fn manager_with_store(spec: HordeSpec, store: RunStore) -> HordeManager {
        let broker = Arc::new(MpscBroker::new());
        let registry = Arc::new(AgentRegistry::new());
        registry
            .register(AgentRecord {
                id: "w1".into(),
                capabilities: vec!["test.a".into(), "test.b".into()],
            })
            .unwrap();
        let federation = Arc::new(FederationOrchestrator::new(registry, broker.clone()));
        HordeManager::new(vec![spec], broker, federation, store)
    }

    /// Integration: a real horde (loaded via `load_horde`) run against a tempdir
    /// file DB; reopening the store simulates a server restart mid-run.
    #[tokio::test]
    async fn file_db_survives_restart_with_mid_run_visible() {
        let dir = tempfile::tempdir().unwrap();
        write_fixture_horde(dir.path());
        let spec = load_horde(dir.path()).unwrap();
        let horde_id = spec.id.clone();
        let db_url = format!("sqlite:{}", dir.path().join("runs.sqlite").display());

        let (completed_id, interrupted_id) = {
            let store = RunStore::open(&db_url).await.unwrap();
            let manager = manager_with_store(spec.clone(), store).await;
            let done = manager
                .start_run(&horde_id, "full run", None, None, RUN_ORIGIN_OPERATOR)
                .await
                .unwrap();
            manager
                .handle_task_finished(&done.run_id, "a", true, Some("out/a.md"), "ok")
                .await;
            manager
                .handle_task_finished(&done.run_id, "b", true, None, "ok")
                .await;
            let mid = manager
                .start_run(&horde_id, "interrupted run", None, None, RUN_ORIGIN_OPERATOR)
                .await
                .unwrap();
            manager
                .handle_task_finished(&mid.run_id, "a", true, None, "ok")
                .await;
            (done.run_id, mid.run_id)
        };

        let store = RunStore::open(&db_url).await.unwrap();
        let manager = manager_with_store(spec, store).await;

        let history = manager.persisted_runs(&horde_id, 50, 0).await.unwrap();
        let ids: Vec<&str> = history.iter().map(|r| r.run_id.as_str()).collect();
        assert!(ids.contains(&completed_id.as_str()), "pre-restart run listed");
        assert!(ids.contains(&interrupted_id.as_str()), "interrupted run listed");

        let completed = manager.persisted_run(&completed_id).await.unwrap().unwrap();
        assert_eq!(completed.status, RunStatus::Done);
        assert!(
            completed
                .steps
                .iter()
                .all(|s| s.status == StepStatus::Succeeded)
        );

        let interrupted = manager
            .persisted_run(&interrupted_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            interrupted.status,
            RunStatus::Running,
            "interrupted run visible as running"
        );
        assert_eq!(interrupted.steps[0].step, "a");
        assert_eq!(interrupted.steps[0].status, StepStatus::Succeeded);
        assert_eq!(interrupted.steps[1].step, "b");
        assert_eq!(interrupted.steps[1].status, StepStatus::Delegating);

        let incomplete = manager.store.incomplete_runs().await.unwrap();
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].run_id, interrupted_id);
    }

    /// DAG fixture with a conditional loop: gen -> verify, verify --pass--> apply,
    /// verify --fail--> gen (max_loops 2). `verify` reads pass/fail from its artifact.
    fn loop_spec(dir: &Path) -> HordeSpec {
        let mut spec = test_spec(dir);
        spec.pipeline = vec!["gen".into(), "verify".into(), "apply".into()];
        spec.sub_agents = vec![sub("gen"), sub("verify"), sub("apply")];
        spec.sub_agents[1].kind = "verify".into();
        let edges = vec![
            HordeEdge { from: "gen".into(), to: "verify".into(), when: None, max_loops: None },
            HordeEdge {
                from: "verify".into(),
                to: "apply".into(),
                when: Some("pass".into()),
                max_loops: None,
            },
            HordeEdge {
                from: "verify".into(),
                to: "gen".into(),
                when: Some("fail".into()),
                max_loops: Some(2),
            },
        ];
        spec.manifest_edges = edges.clone();
        spec.execution_graph = resolve_execution_graph(&spec.pipeline, Some(&edges)).unwrap();
        spec
    }

    fn write_verify_artifact(dir: &Path, name: &str, status: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, format!("---\nstatus: {status}\n---\nverify report\n")).unwrap();
        p.display().to_string()
    }

    async fn loop_manager(dir: &Path, store: RunStore) -> HordeManager {
        let broker = Arc::new(MpscBroker::new());
        let registry = Arc::new(AgentRegistry::new());
        registry
            .register(AgentRecord {
                id: "w1".into(),
                capabilities: vec!["test.gen".into(), "test.verify".into(), "test.apply".into()],
            })
            .unwrap();
        let federation = Arc::new(FederationOrchestrator::new(registry, broker.clone()));
        HordeManager::new(vec![loop_spec(dir)], broker, federation, store)
    }

    /// Insert an interrupted run directly at the store level (simulates a run
    /// whose server died while `current` was in flight).
    async fn seed_interrupted_run(
        store: &RunStore,
        run_id: &str,
        origin: &str,
        status: RunStatus,
        done_steps: &[(&str, &str)],
        in_flight: Option<&str>,
        pending: &[&str],
    ) {
        store
            .create_run(&NewRun {
                run_id: run_id.into(),
                horde_id: "test-horde".into(),
                prompt: "seeded".into(),
                source: None,
                question: "q?".into(),
                manifest_snapshot: Some(serde_json::json!({
                    "pipeline": ["a", "b"],
                    "manifest_edges": [],
                })),
                origin: origin.into(),
            })
            .await
            .unwrap();
        for (name, artifact) in done_steps {
            store
                .upsert_step(
                    run_id,
                    &StepUpdate {
                        step: (*name).into(),
                        agent_id: format!("agent-{name}"),
                        task_id: format!("test-horde::{run_id}::{name}"),
                        status: StepStatus::Succeeded,
                        attempt: 1,
                        outcome: Some("pass".into()),
                        artifact: Some((*artifact).into()),
                        summary: Some("ok".into()),
                    },
                )
                .await
                .unwrap();
        }
        if let Some(name) = in_flight {
            store
                .upsert_step(
                    run_id,
                    &StepUpdate {
                        step: name.into(),
                        agent_id: format!("agent-{name}"),
                        task_id: format!("test-horde::{run_id}::{name}"),
                        status: StepStatus::Delegating,
                        attempt: 1,
                        outcome: None,
                        artifact: None,
                        summary: None,
                    },
                )
                .await
                .unwrap();
        }
        for name in pending {
            store
                .upsert_step(
                    run_id,
                    &StepUpdate {
                        step: (*name).into(),
                        agent_id: format!("agent-{name}"),
                        task_id: format!("test-horde::{run_id}::{name}"),
                        status: StepStatus::Pending,
                        attempt: 1,
                        outcome: None,
                        artifact: None,
                        summary: None,
                    },
                )
                .await
                .unwrap();
        }
        store.update_run_status(run_id, status, None).await.unwrap();
    }

    /// Kill/restart around a linear horde: the resumed run re-delegates the
    /// interrupted step as a new attempt and never regenerates the completed
    /// step's artifact.
    #[tokio::test]
    async fn resume_redelegates_interrupted_step_and_keeps_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let db_url = format!("sqlite:{}", dir.path().join("runs.sqlite").display());

        let run_id = {
            let store = RunStore::open(&db_url).await.unwrap();
            let manager = test_manager(dir.path(), true).await;
            let manager =
                HordeManager::new(vec![test_spec(dir.path())], manager.broker.clone(), manager.federation.clone(), store);
            let record = manager
                .start_run("test-horde", "kill me", None, None, RUN_ORIGIN_OPERATOR)
                .await
                .unwrap();
            manager
                .handle_task_finished(&record.run_id, "a", true, Some("out/a.md"), "ok")
                .await;
            record.run_id
            // manager dropped here = server killed while step b was in flight
        };

        let store = RunStore::open(&db_url).await.unwrap();
        let manager = manager_with_store(test_spec(dir.path()), store).await;
        let resumed = manager.resume_run(&run_id).await.unwrap();
        assert_eq!(resumed.status, RunStatus::Running);
        assert_eq!(resumed.resume_count, 1);

        let persisted = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(persisted.resume_count, 1);
        let a = step(&persisted, "a");
        assert_eq!(a.status, StepStatus::Succeeded, "completed step untouched");
        assert_eq!(a.artifact.as_deref(), Some("out/a.md"), "artifact not regenerated");
        assert_eq!(a.attempt, 1);
        let b = step(&persisted, "b");
        assert_eq!(b.status, StepStatus::Delegating, "interrupted step re-delegated");
        assert_eq!(b.attempt, 2, "interrupted attempt counted as failed");
        assert!(
            persisted
                .events
                .iter()
                .any(|e| e.get("kind").and_then(|k| k.as_str()) == Some("run_resumed")),
            "resume marker recorded in the run feed"
        );

        // Resumed run completes through the normal advance path.
        manager
            .handle_task_finished(&run_id, "b", true, None, "ok")
            .await;
        let done = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(done.status, RunStatus::Done);
    }

    /// Startup scan: non-operator runs auto-resume; operator runs are only
    /// surfaced as resumable (with the interruption recorded in the feed).
    #[tokio::test]
    async fn resume_scan_auto_resumes_trigger_and_surfaces_operator_runs() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::open("sqlite::memory:").await.unwrap();
        seed_interrupted_run(&store, "run-op", RUN_ORIGIN_OPERATOR, RunStatus::Running, &[("a", "out/a.md")], Some("b"), &[])
            .await;
        seed_interrupted_run(&store, "run-trig", "trigger", RunStatus::Running, &[("a", "out/a.md")], Some("b"), &[])
            .await;
        let manager = manager_with_store(test_spec(dir.path()), store).await;

        manager.resume_scan().await;

        let trig = manager.store.get_run("run-trig").await.unwrap().unwrap();
        assert_eq!(trig.resume_count, 1, "trigger run auto-resumed");
        assert_eq!(step(&trig, "b").status, StepStatus::Delegating);
        assert_eq!(step(&trig, "b").attempt, 2);

        let op = manager.store.get_run("run-op").await.unwrap().unwrap();
        assert_eq!(op.resume_count, 0, "operator run left for on-demand resume");
        assert_eq!(step(&op, "b").status, StepStatus::Delegating, "store state untouched");
        assert!(
            op.events
                .iter()
                .any(|e| e.get("kind").and_then(|k| k.as_str()) == Some("run_interrupted")),
            "interruption recorded"
        );

        let listed = manager.persisted_runs("test-horde", 50, 0).await.unwrap();
        let op_listed = listed.iter().find(|r| r.run_id == "run-op").unwrap();
        assert!(op_listed.resumable, "operator run listed as resumable");
        let trig_listed = listed.iter().find(|r| r.run_id == "run-trig").unwrap();
        assert!(!trig_listed.resumable, "auto-resumed run is active again");
    }

    /// Guard rail: a run whose resumes keep failing (no worker registered) goes
    /// to `error` with a clear reason once the attempt cap is spent.
    #[tokio::test]
    async fn resume_cap_exhausted_errors_run_with_reason() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::open("sqlite::memory:").await.unwrap();
        seed_interrupted_run(&store, "run-1", RUN_ORIGIN_OPERATOR, RunStatus::Running, &[], Some("a"), &["b"])
            .await;
        // No worker registered → every delegation fails.
        let broker = Arc::new(MpscBroker::new());
        let registry = Arc::new(AgentRegistry::new());
        let federation = Arc::new(FederationOrchestrator::new(registry, broker.clone()));
        let manager = HordeManager::new(vec![test_spec(dir.path())], broker, federation, store);
        assert_eq!(manager.resume_max_attempts, DEFAULT_RESUME_MAX_ATTEMPTS);

        let first = manager.resume_run("run-1").await;
        assert!(first.is_err(), "no worker → resume fails");
        let after_first = manager.store.get_run("run-1").await.unwrap().unwrap();
        assert_eq!(after_first.resume_count, 1);
        assert_eq!(after_first.status, RunStatus::Running, "still resumable below the cap");

        let second = manager.resume_run("run-1").await;
        assert!(second.is_err());
        let after_second = manager.store.get_run("run-1").await.unwrap().unwrap();
        assert_eq!(after_second.resume_count, 2);
        assert_eq!(after_second.status, RunStatus::Error, "cap spent → error");
        assert!(
            after_second.result.as_deref().unwrap_or("").contains("resume failed"),
            "reason recorded: {:?}",
            after_second.result
        );

        let third = manager.resume_run("run-1").await;
        assert!(third.unwrap_err().contains("not resumable"));
    }

    /// Awaiting-input runs are durable by construction: the scan leaves them
    /// alone, the API lists them as resumable, and resume delegates on demand.
    #[tokio::test]
    async fn awaiting_input_run_is_resumable_on_demand() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::open("sqlite::memory:").await.unwrap();
        seed_interrupted_run(&store, "run-wait", RUN_ORIGIN_OPERATOR, RunStatus::AwaitingInput, &[], None, &["a", "b"])
            .await;
        let manager = manager_with_store(test_spec(dir.path()), store).await;

        manager.resume_scan().await;
        let after_scan = manager.store.get_run("run-wait").await.unwrap().unwrap();
        assert_eq!(after_scan.status, RunStatus::AwaitingInput, "scan leaves awaiting runs parked");
        assert_eq!(after_scan.resume_count, 0);

        let listed = manager.persisted_runs("test-horde", 50, 0).await.unwrap();
        assert!(listed.iter().find(|r| r.run_id == "run-wait").unwrap().resumable);

        let resumed = manager.resume_run("run-wait").await.unwrap();
        assert_eq!(resumed.status, RunStatus::Running);
        let persisted = manager.store.get_run("run-wait").await.unwrap().unwrap();
        assert_eq!(persisted.status, RunStatus::Running);
        assert_eq!(step(&persisted, "a").status, StepStatus::Delegating);
    }

    /// Crash between the last step finishing and run completion: resume closes
    /// the run out instead of re-delegating anything.
    #[tokio::test]
    async fn resume_completes_run_when_all_steps_succeeded() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::open("sqlite::memory:").await.unwrap();
        seed_interrupted_run(
            &store,
            "run-done",
            RUN_ORIGIN_OPERATOR,
            RunStatus::Running,
            &[("a", "out/a.md"), ("b", "out/b.md")],
            None,
            &[],
        )
        .await;
        let manager = manager_with_store(test_spec(dir.path()), store).await;

        let resumed = manager.resume_run("run-done").await.unwrap();
        assert_eq!(resumed.status, RunStatus::Done);
        let persisted = manager.store.get_run("run-done").await.unwrap().unwrap();
        assert_eq!(persisted.status, RunStatus::Done);
        assert!(persisted.finished_at.is_some());
    }

    /// Kill/restart mid-loop in a conditional DAG: loop counts survive, the
    /// completed generator attempt is not re-run, and no extra loop iteration
    /// is spent.
    #[tokio::test]
    async fn resume_mid_loop_keeps_loop_counts_intact() {
        let dir = tempfile::tempdir().unwrap();
        let db_url = format!("sqlite:{}", dir.path().join("runs.sqlite").display());
        let fail_artifact = write_verify_artifact(dir.path(), "verify-fail.md", "fail");

        let run_id = {
            let store = RunStore::open(&db_url).await.unwrap();
            let manager = loop_manager(dir.path(), store).await;
            let record = manager
                .start_run("test-horde", "loop then die", None, None, RUN_ORIGIN_OPERATOR)
                .await
                .unwrap();
            // Iteration 1: gen ok, verify says fail → loop back resets gen+verify.
            manager
                .handle_task_finished(&record.run_id, "gen", true, Some("out/gen-1.md"), "ok")
                .await;
            manager
                .handle_task_finished(&record.run_id, "verify", true, Some(&fail_artifact), "checked")
                .await;
            // Iteration 2: gen ok again; server dies while verify is in flight.
            manager
                .handle_task_finished(&record.run_id, "gen", true, Some("out/gen-2.md"), "ok")
                .await;
            let mid = manager.store.get_run(&record.run_id).await.unwrap().unwrap();
            assert_eq!(mid.loop_counts.get("verify->gen"), Some(&1));
            assert_eq!(step(&mid, "verify").status, StepStatus::Delegating);
            record.run_id
        };

        let store = RunStore::open(&db_url).await.unwrap();
        let manager = loop_manager(dir.path(), store).await;
        let resumed = manager.resume_run(&run_id).await.unwrap();
        assert_eq!(resumed.status, RunStatus::Running);

        let persisted = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(
            persisted.loop_counts.get("verify->gen"),
            Some(&1),
            "loop counts intact — no extra iteration granted"
        );
        let gen_step = step(&persisted, "gen");
        assert_eq!(gen_step.status, StepStatus::Succeeded, "second gen attempt not re-run");
        assert_eq!(gen_step.artifact.as_deref(), Some("out/gen-2.md"));
        assert_eq!(gen_step.attempt, 2, "attempt counter reflects the loop, not the resume");
        let verify = step(&persisted, "verify");
        assert_eq!(verify.status, StepStatus::Delegating, "resume re-delegated verify");
        assert_eq!(verify.attempt, 3, "loop attempt + interrupted attempt");
        assert_eq!(step(&persisted, "apply").status, StepStatus::Pending);

        // Verify passes this time → the loop routes forward and the run completes.
        let pass_artifact = write_verify_artifact(dir.path(), "verify-pass.md", "pass");
        manager
            .handle_task_finished(&run_id, "verify", true, Some(&pass_artifact), "checked")
            .await;
        manager
            .handle_task_finished(&run_id, "apply", true, Some("out/apply.md"), "ok")
            .await;
        let done = manager.store.get_run(&run_id).await.unwrap().unwrap();
        assert_eq!(done.status, RunStatus::Done);
        assert_eq!(done.loop_counts.get("verify->gen"), Some(&1));
    }

    #[test]
    fn api_status_vocabulary_stays_legacy() {
        assert_eq!(api_run_status(RunStatus::Done), "completed");
        assert_eq!(api_run_status(RunStatus::Error), "failed");
        assert_eq!(api_run_status(RunStatus::Running), "running");
        assert_eq!(api_step_status(StepStatus::Succeeded), "success");
        assert_eq!(api_step_status(StepStatus::Pending), "pending");
        assert_eq!(api_step_status(StepStatus::Delegating), "delegating");
    }
}
