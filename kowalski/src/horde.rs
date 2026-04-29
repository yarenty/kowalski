//! Horde catalog, in-memory run state, and server-side orchestrator task.
//!
//! A *horde* is a markdown-defined multi-agent workflow with one worker per sub-agent.
//! The orchestrator subscribes to the federation broker, advances the run pipeline as each
//! sub-agent's worker reports a [`crate::core::AclMessage::TaskFinished`], and emits
//! lifecycle events (`RunStarted`, `TaskAssigned`, `AgentMessage`, `RunFinished`, `RunFailed`).

use kowalski_core::federation::{
    AclEnvelope, AclMessage, FederationOrchestrator, MpscBroker,
};
use kowalski_core::MessageBroker;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const DEFAULT_TOPIC: &str = "federation";
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
    pub default_question: Option<String>,
    #[serde(default)]
    pub default_topic: Option<String>,
    #[serde(default)]
    pub artifacts_root: Option<String>,
    #[serde(default)]
    pub delivery_title: Option<String>,
    #[serde(default)]
    pub delivery_note: Option<String>,
    #[serde(default)]
    pub delivery_root_rel: Option<String>,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct HordeSpec {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub capability_prefix: String,
    pub pipeline: Vec<String>,
    pub default_question: String,
    pub topic: String,
    pub artifacts_root: PathBuf,
    pub delivery_title: String,
    pub delivery_note: String,
    pub delivery_root_rel: String,
    pub root_path: PathBuf,
    pub sub_agents: Vec<SubAgentSpec>,
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
    let prefix = meta.capability_prefix.clone().unwrap_or_else(|| meta.id.clone());

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

    Ok(HordeSpec {
        id: meta.id,
        display_name: meta.display_name,
        description: meta.description,
        capability_prefix: prefix,
        pipeline: meta.pipeline,
        default_question: meta
            .default_question
            .unwrap_or_else(|| "What changed?".to_string()),
        topic: meta.default_topic.unwrap_or_else(|| DEFAULT_TOPIC.to_string()),
        artifacts_root: root.join(meta.artifacts_root.unwrap_or_else(|| ".".to_string())),
        delivery_title: meta
            .delivery_title
            .unwrap_or_else(|| "Final delivery".to_string()),
        delivery_note: meta.delivery_note.unwrap_or_else(|| {
            "Review generated artifacts and import the horde output folder into your target knowledge system."
                .to_string()
        }),
        delivery_root_rel: meta
            .delivery_root_rel
            .unwrap_or_else(|| "wiki".to_string()),
        root_path: root.to_path_buf(),
        sub_agents,
    })
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
            if let Ok(spec) = load_horde(r) {
                if seen.insert(spec.id.clone()) {
                    out.push(spec);
                }
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
                        Err(err) => log::warn!(
                            "horde load failed at {}: {}",
                            p.display(),
                            err
                        ),
                    }
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
pub struct RunStepRecord {
    pub step: String,
    pub agent_id: String,
    pub task_id: String,
    pub status: String,
    pub artifact: Option<String>,
    pub summary: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub run_id: String,
    pub horde_id: String,
    pub prompt: String,
    pub source: Option<String>,
    pub question: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub current_step_index: usize,
    pub steps: Vec<RunStepRecord>,
    pub events: Vec<serde_json::Value>,
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
}

impl HordeManager {
    pub fn new(
        specs: Vec<HordeSpec>,
        broker: Arc<MpscBroker>,
        federation: Arc<FederationOrchestrator>,
    ) -> Self {
        Self {
            specs: Arc::new(specs),
            runs: Arc::new(Mutex::new(RunRegistry::default())),
            broker,
            federation,
            orchestrator_id: federation_orchestrator_id(),
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
        let payload = json!({
            "horde": spec.id,
            "run_id": run.run_id,
            "step": step,
            "kind": kind,
            "source": run.source,
            "question": run.question,
            "previous_artifact": previous_artifact,
            "horde_root": spec.root_path.display().to_string(),
        });
        payload.to_string()
    }

    /// Start a new horde run: register, emit RunStarted, delegate first step.
    pub async fn start_run(
        &self,
        horde_id: &str,
        prompt: &str,
        source: Option<&str>,
        question: Option<&str>,
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
            status: "running".to_string(),
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
                    status: "pending".to_string(),
                    artifact: None,
                    summary: None,
                    started_at: started_at.clone(),
                    finished_at: None,
                })
                .collect(),
            events: Vec::new(),
        };

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
        self.publish(&env).await;

        {
            let mut runs = self.runs.lock().await;
            runs.runs.insert(run_id.clone(), record.clone());
        }

        if let Err(e) = self.delegate_step(&spec, &run_id, 0, None).await {
            self.fail_run(&spec, &run_id, &format!("delegate first step failed: {}", e), Some(&spec.pipeline[0])).await;
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
        index: usize,
        previous_artifact: Option<&str>,
    ) -> Result<(), String> {
        let step_name = spec
            .pipeline
            .get(index)
            .ok_or_else(|| "pipeline index out of range".to_string())?
            .clone();
        let sub = spec
            .sub_agent(&step_name)
            .ok_or_else(|| format!("missing sub-agent {} in horde {}", step_name, spec.id))?
            .clone();
        let task_id = self.task_id(&spec.id, run_id, &step_name);

        let instruction;
        let assigned_envelope;
        let run_for_log;
        {
            let mut runs = self.runs.lock().await;
            let run = runs
                .runs
                .get_mut(run_id)
                .ok_or_else(|| format!("run {} no longer tracked", run_id))?;
            run.current_step_index = index;
            if let Some(step) = run.steps.get_mut(index) {
                step.status = "delegating".to_string();
                step.task_id = task_id.clone();
                step.agent_id = sub.default_agent_id.clone();
                step.started_at = now_ts();
            }
            instruction = self.build_instruction(spec, run, &step_name, previous_artifact);
            let assigned_msg = AclMessage::TaskAssigned {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                step: step_name.clone(),
                from: self.orchestrator_id.clone(),
                to: sub.default_agent_id.clone(),
                task_id: task_id.clone(),
                instruction: instruction.clone(),
            };
            assigned_envelope = self.build_envelope(&spec.topic, assigned_msg);
            run.events.push(envelope_summary(&assigned_envelope));
            run_for_log = run.clone();
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

        let next_index;
        {
            let mut runs = self.runs.lock().await;
            let Some(run) = runs.runs.get_mut(run_id) else {
                return;
            };
            if matches!(run.status.as_str(), "completed" | "failed") {
                return;
            }
            let already_finalized = run
                .steps
                .iter()
                .find(|s| s.step == step)
                .map(|s| matches!(s.status.as_str(), "success" | "failed"))
                .unwrap_or(false);
            if already_finalized {
                return;
            }
            if let Some(step_record) = run.steps.iter_mut().find(|s| s.step == step) {
                step_record.status = if success { "success".into() } else { "failed".into() };
                step_record.artifact = artifact.map(ToString::to_string);
                step_record.summary = Some(summary.to_string());
                step_record.finished_at = Some(now_ts());
            }
            run.events.push(json!({
                "kind": "task_finished",
                "step": step,
                "success": success,
                "artifact": artifact,
                "summary": summary,
                "ts": now_ts(),
            }));
            next_index = run
                .steps
                .iter()
                .position(|s| s.step == step)
                .map(|i| i + 1)
                .unwrap_or(spec.pipeline.len());
        }

        if !success {
            self.fail_run(&spec, run_id, &format!("step {} failed: {}", step, summary), Some(step)).await;
            return;
        }

        if next_index < spec.pipeline.len() {
            let prev = artifact.map(ToString::to_string);
            if let Err(e) = self
                .delegate_step(&spec, run_id, next_index, prev.as_deref())
                .await
            {
                self.fail_run(&spec, run_id, &e, Some(&spec.pipeline[next_index])).await;
            }
        } else {
            self.complete_run(&spec, run_id).await;
        }
    }

    async fn complete_run(&self, spec: &HordeSpec, run_id: &str) {
        let artifacts: Vec<(String, String)> = {
            let mut runs = self.runs.lock().await;
            let Some(run) = runs.runs.get_mut(run_id) else {
                return;
            };
            run.status = "completed".into();
            run.finished_at = Some(now_ts());
            run.steps
                .iter()
                .filter_map(|s| s.artifact.clone().map(|a| (s.step.clone(), a)))
                .collect()
        };
        let env = self.build_envelope(
            &spec.topic,
            AclMessage::RunFinished {
                run_id: run_id.to_string(),
                horde: spec.id.clone(),
                artifacts: artifacts.clone(),
                text: Some(format!(
                    "{} run completed; {} artifact(s).",
                    spec.display_name,
                    artifacts.len()
                )),
            },
        );
        {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                run.events.push(envelope_summary(&env));
            }
        }
        self.publish(&env).await;
    }

    async fn fail_run(&self, spec: &HordeSpec, run_id: &str, reason: &str, step: Option<&str>) {
        {
            let mut runs = self.runs.lock().await;
            if let Some(run) = runs.runs.get_mut(run_id) {
                run.status = "failed".into();
                run.finished_at = Some(now_ts());
                run.events.push(json!({
                    "kind": "run_failed",
                    "reason": reason,
                    "step": step,
                    "ts": now_ts(),
                }));
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
            run.events.push(serde_json::to_value(event).unwrap_or(json!({})));
        }
    }

    pub async fn snapshot(&self, run_id: &str) -> Option<RunRecord> {
        let runs = self.runs.lock().await;
        runs.runs.get(run_id).cloned()
    }

    pub async fn list_runs(&self) -> Vec<RunRecord> {
        let runs = self.runs.lock().await;
        let mut out: Vec<RunRecord> = runs.runs.values().cloned().collect();
        out.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        out
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
        } => {
            manager
                .handle_task_finished(run_id, step, *success, artifact.as_deref(), summary)
                .await;
        }
        AclMessage::AgentMessage { run_id, .. }
        | AclMessage::TaskStarted { run_id, .. }
        | AclMessage::TaskAssigned { run_id, .. } => {
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
                    .handle_task_finished(
                        &run_id,
                        &step,
                        *success,
                        artifact.as_deref(),
                        outcome,
                    )
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
