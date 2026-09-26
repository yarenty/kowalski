//! Rookery HTTP API — conversational horde builder (`/api/rookery/*`).

use axum::extract::Path as AxumPath;
use axum::Extension;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use axum::Json;
use futures::StreamExt;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::conversation::Message;
use kowalski_core::error::KowalskiError;
use kowalski_core::llm::LLMProvider;
use kowalski_core::rookery::{
    normalize_draft, parse_draft_from_assistant, run_ops_phase, slugify_horde_id, validate_draft,
    validate_horde_tree, write_horde_tree,
    HordeBirthSpec, InterviewConfig, OpsModel, OpsPhase, RookeryDraft,
};
use kowalski_core::template::agent::TemplateAgent;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use kowalski_core::config::default_model as determine_model;

const BUILDER_PROMPT_REL: &str = "resources/prompts/rookery/builder.md";

/// The builder prompt compiled into the binary, used when no override file is found on disk
/// (a `cargo install`ed server has no repository next to it).
const BUILDER_PROMPT_EMBEDDED: &str = include_str!("../resources/prompts/rookery/builder.md");
const PROPOSE_USER_MESSAGE: &str = "Based on our conversation so far, emit ONLY a single ```toml code block with the complete horde draft. No other prose.\n\nRequired top-level: `id`, `display_name`, `description`, `pipeline` (array of step names), and `[[penguins]]` rows with at least `name`, `description`, `prompt_body`, `output`. Optional per penguin: `kind` (inferred from `name` if omitted), `display_name`, `context_paths`, `inputs`.\n\nID rules: `id` and every penguin `name` / pipeline entry must be lowercase ASCII kebab-case (`a-z`, `0-9`, hyphens only). Human titles go in `display_name`, not in `name`.\n\nExample:\n```toml\nid = \"my-horde\"\ndisplay_name = \"My Horde\"\ndescription = \"…\"\npipeline = [\"ingest\", \"deliver\"]\n\n[[penguins]]\nname = \"ingest\"\ndescription = \"…\"\nprompt_body = \"…\"\noutput = \"debug/raw/\"\n\n[[penguins]]\nname = \"deliver\"\ndescription = \"…\"\nprompt_body = \"…\"\noutput = \"HANDOFF.md\"\n```\n\nYou may use ```json instead if needed; omitting `kind` is OK when the step `name` is ingest/deliver/ask/lint.";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RookerySessionStatus {
    Interviewing,
    Proposed,
    Born,
}

#[derive(Clone, Debug)]
pub struct RookerySession {
    pub id: String,
    pub conversation_id: String,
    pub status: RookerySessionStatus,
    pub draft: Option<RookeryDraft>,
    pub summary: Option<String>,
    pub horde_root: Option<PathBuf>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

pub struct RookeryStore {
    pub agent: TemplateAgent,
    pub model: String,
    pub output_root: PathBuf,
    /// Interview policy (`[rookery]` in `config.toml`): per-turn ops cap, structured output,
    /// `replace_draft` gate.
    pub interview: InterviewConfig,
    /// Directory where session snapshots are persisted so they survive a server restart.
    persist_dir: PathBuf,
    sessions: HashMap<String, RookerySession>,
}

impl RookeryStore {
    pub fn get(&self, id: &str) -> Option<&RookerySession> {
        self.sessions.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut RookerySession> {
        self.sessions.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<RookerySession> {
        let removed = self.sessions.remove(id);
        if removed.is_some() {
            let path = session_file_path(&self.persist_dir, id);
            if let Err(e) = std::fs::remove_file(&path)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                log::warn!(
                    "rookery: failed to delete session file {}: {}",
                    path.display(),
                    e
                );
            }
        }
        removed
    }

    pub fn create_session(&mut self) -> RookerySession {
        let id = format!("rookery-{}", Uuid::new_v4());
        let conversation_id = self.agent.start_conversation(&self.model);
        let now = now_ms();
        let session = RookerySession {
            id: id.clone(),
            conversation_id,
            status: RookerySessionStatus::Interviewing,
            draft: None,
            summary: None,
            horde_root: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.sessions.insert(id.clone(), session.clone());
        self.persist(&id);
        session
    }

    fn touch(session: &mut RookerySession) {
        session.updated_at_ms = now_ms();
    }

    /// Write a session snapshot (metadata + draft + chat transcript) to disk.
    ///
    /// The server is the source of truth for sessions; the UI no longer needs to round-trip
    /// the draft/history through the browser to recover after a restart (see `PLAN.md` §R1).
    pub fn persist(&self, session_id: &str) {
        let Some(session) = self.sessions.get(session_id) else {
            return;
        };
        let transcript = self
            .agent
            .get_conversation(&session.conversation_id)
            .map(|c| {
                c.messages
                    .iter()
                    .filter(|m| m.role == "user" || m.role == "assistant")
                    .filter(|m| !m.content.trim().is_empty())
                    .map(|m| PersistedTurn {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let persisted = PersistedSession {
            id: session.id.clone(),
            status: session.status.clone(),
            draft: session.draft.clone(),
            summary: session.summary.clone(),
            horde_root: session.horde_root.clone(),
            created_at_ms: session.created_at_ms,
            updated_at_ms: session.updated_at_ms,
            transcript,
        };
        if let Err(e) = write_session_file(&self.persist_dir, &persisted) {
            log::warn!("rookery: failed to persist session {}: {}", session_id, e);
        }
    }
}

/// Relative directory (under the config dir) for persisted Rookery session snapshots.
const ROOKERY_STATE_DIR_REL: &str = "db/rookery";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedTurn {
    role: String,
    content: String,
}

/// On-disk snapshot of a Rookery session (one YAML file per session).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedSession {
    id: String,
    status: RookerySessionStatus,
    #[serde(default)]
    draft: Option<RookeryDraft>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    horde_root: Option<PathBuf>,
    created_at_ms: u64,
    updated_at_ms: u64,
    #[serde(default)]
    transcript: Vec<PersistedTurn>,
}

fn session_file_path(dir: &Path, session_id: &str) -> PathBuf {
    dir.join(format!("{session_id}.yaml"))
}

fn write_session_file(dir: &Path, session: &PersistedSession) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let yaml = serde_yaml::to_string(session)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(session_file_path(dir, &session.id), yaml)
}

/// Directory used to persist Rookery sessions (`KOWALSKI_ROOKERY_STATE` overrides; else `db/rookery`).
pub fn default_rookery_state_dir(config_dir: Option<&Path>) -> PathBuf {
    if let Ok(env) = std::env::var("KOWALSKI_ROOKERY_STATE") {
        let p = env.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = config_dir
        .map(|c| c.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join(ROOKERY_STATE_DIR_REL)
}

/// Load persisted sessions from disk, rebuilding each conversation in the agent.
fn load_persisted_sessions(
    agent: &mut TemplateAgent,
    model: &str,
    dir: &Path,
) -> HashMap<String, RookerySession> {
    let mut map = HashMap::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return map,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let persisted: PersistedSession = match std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_yaml::from_str(&t).ok())
        {
            Some(p) => p,
            None => {
                log::warn!("rookery: skipping unreadable session file {}", path.display());
                continue;
            }
        };
        let conversation_id = agent.start_conversation(model);
        if !persisted.transcript.is_empty() {
            let messages = persisted
                .transcript
                .iter()
                .map(|t| Message::text(&t.role, &t.content))
                .collect::<Vec<_>>();
            if let Err(e) = agent.replace_conversation_messages(&conversation_id, messages) {
                log::warn!(
                    "rookery: failed to restore transcript for {}: {}",
                    persisted.id,
                    e
                );
            }
        }
        let session = RookerySession {
            id: persisted.id.clone(),
            conversation_id,
            status: persisted.status,
            draft: persisted.draft,
            summary: persisted.summary,
            horde_root: persisted.horde_root,
            created_at_ms: persisted.created_at_ms,
            updated_at_ms: persisted.updated_at_ms,
        };
        map.insert(session.id.clone(), session);
    }
    if !map.is_empty() {
        log::info!(
            "rookery: restored {} session(s) from {}",
            map.len(),
            dir.display()
        );
    }
    map
}

pub async fn new_rookery_store(
    config: &Config,
    config_path: &Path,
) -> Result<Arc<Mutex<RookeryStore>>, Box<dyn std::error::Error>> {
    let prompt = load_builder_prompt(config_path)?;
    let mut agent = TemplateAgent::new(config.clone()).await?;
    agent = agent.with_system_prompt(&prompt);
    let model = determine_model(config);
    let output_root = default_rookery_output_root(config_path.parent());
    let persist_dir = default_rookery_state_dir(config_path.parent());
    if let Err(e) = std::fs::create_dir_all(&persist_dir) {
        log::warn!(
            "rookery: cannot create state dir {}: {}",
            persist_dir.display(),
            e
        );
    }
    let sessions = load_persisted_sessions(&mut agent, &model, &persist_dir);
    Ok(Arc::new(Mutex::new(RookeryStore {
        agent,
        model,
        output_root,
        interview: interview_config_from(config),
        persist_dir,
        sessions,
    })))
}

/// `[rookery]` in `config.toml` — interview policy knobs, all optional:
/// `max_ops_per_turn` (default 12), `structured_output` (default true; effective only when the
/// provider also opts in via `[llm] structured_output`), `allow_replace_draft` (default false).
fn interview_config_from(config: &Config) -> InterviewConfig {
    let mut cfg = InterviewConfig::default();
    if let Some(section) = config.additional.get("rookery").and_then(|v| v.as_object()) {
        if let Some(v) = section.get("max_ops_per_turn").and_then(|v| v.as_u64()) {
            cfg.max_ops = v.max(1) as usize;
        }
        if let Some(v) = section.get("structured_output").and_then(|v| v.as_bool()) {
            cfg.structured_output = v;
        }
        if let Some(v) = section.get("allow_replace_draft").and_then(|v| v.as_bool()) {
            cfg.allow_replace_draft = v;
        }
    }
    cfg
}

pub fn default_rookery_output_root(config_dir: Option<&Path>) -> PathBuf {
    if let Ok(env) = std::env::var("KOWALSKI_ROOKERY_OUTPUT") {
        let p = env.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Some(c) = config_dir {
        if let Some(parent) = c.parent() {
            return parent.join("examples");
        }
        return c.join("examples");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("examples")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Load the builder system prompt: an override file at `resources/prompts/rookery/builder.md`
/// next to the config, one level up, or in the working directory wins; otherwise the prompt
/// compiled into the binary.
pub fn load_builder_prompt(config_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(parent) = config_path.parent() {
        candidates.push(parent.join(BUILDER_PROMPT_REL));
        if let Some(gp) = parent.parent() {
            candidates.push(gp.join(BUILDER_PROMPT_REL));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(BUILDER_PROMPT_REL));
    }

    for p in &candidates {
        if p.is_file() {
            return Ok(std::fs::read_to_string(p)?);
        }
    }
    Ok(BUILDER_PROMPT_EMBEDDED.to_string())
}

#[derive(Serialize)]
pub struct RookerySessionResponse {
    pub session_id: String,
    pub conversation_id: String,
    pub status: RookerySessionStatus,
    pub draft: Option<RookeryDraft>,
    pub summary: Option<String>,
    pub pipeline: Vec<String>,
    pub horde_root: Option<String>,
    pub output_root: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl RookerySessionResponse {
    pub fn from_session(session: &RookerySession, output_root: &Path) -> Self {
        let pipeline = session
            .draft
            .as_ref()
            .map(|d| d.pipeline.clone())
            .unwrap_or_default();
        Self {
            session_id: session.id.clone(),
            conversation_id: session.conversation_id.clone(),
            status: session.status.clone(),
            draft: session.draft.clone(),
            summary: session.summary.clone(),
            pipeline,
            horde_root: session
                .horde_root
                .as_ref()
                .map(|p| p.display().to_string()),
            output_root: output_root.display().to_string(),
            created_at_ms: session.created_at_ms,
            updated_at_ms: session.updated_at_ms,
        }
    }
}

#[derive(Serialize)]
pub struct CreateSessionResponse {
    pub session: RookerySessionResponse,
}

/// Optional restore payload when the UI reconnects after a server restart.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CreateSessionBody {
    #[serde(default)]
    pub history: Vec<RookeryHistoryTurn>,
    #[serde(default)]
    pub draft: Option<RookeryDraft>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub status: Option<RookerySessionStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RookeryHistoryTurn {
    pub role: String,
    pub content: String,
}

fn history_to_messages(history: &[RookeryHistoryTurn]) -> Vec<Message> {
    history
        .iter()
        .filter(|t| {
            let role = t.role.as_str();
            (role == "user" || role == "assistant") && !t.content.trim().is_empty()
        })
        .map(|t| Message::text(&t.role, &t.content))
        .collect()
}

fn apply_session_restore(session: &mut RookerySession, body: &CreateSessionBody) {
    if let Some(draft) = body.draft.clone() {
        let mut d = draft;
        normalize_draft(&mut d);
        session.draft = Some(d);
    }
    if let Some(summary) = &body.summary {
        session.summary = Some(summary.clone());
    }
    if let Some(status) = &body.status {
        session.status = status.clone();
    } else if session.draft.is_some() {
        session.status = RookerySessionStatus::Proposed;
    }
}

#[derive(Deserialize)]
pub struct RookeryChatBody {
    pub message: String,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Serialize)]
pub struct RookeryChatResponse {
    pub reply: String,
    pub session: RookerySessionResponse,
    /// Delta ops applied to the draft this turn (guided interview).
    pub ops_applied: usize,
    /// First rejected op of the turn's batch, if any (prefix-apply kept the draft consistent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ops_error: Option<String>,
}

#[derive(Serialize)]
pub struct ProposeResponse {
    pub session: RookerySessionResponse,
    pub parse_error: Option<String>,
}

#[derive(Deserialize)]
pub struct GiveBirthBody {
    #[serde(default)]
    pub output_root: Option<String>,
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Serialize)]
pub struct GiveBirthResponse {
    pub ok: bool,
    pub horde_root: String,
    pub horde_id: String,
    pub validate_ok: bool,
    pub validate_errors: Option<String>,
    pub session: RookerySessionResponse,
}

/// Partial update to one penguin in the session draft (draft buffer until save / give birth).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PatchPenguinBody {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub prompt_body: Option<String>,
    #[serde(default)]
    pub agent_body: Option<String>,
    #[serde(default)]
    pub clear_agent_body: bool,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub context_paths: Option<Vec<String>>,
    #[serde(default)]
    pub tool_ids: Option<Vec<String>>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub clear_model_id: bool,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub clear_avatar: bool,
}

#[derive(Serialize)]
pub struct PatchPenguinResponse {
    pub session: RookerySessionResponse,
}

#[derive(Serialize)]
pub struct SaveHordeResponse {
    pub ok: bool,
    pub horde_root: String,
    pub horde_id: String,
    pub validate_ok: bool,
    pub validate_errors: Option<String>,
    pub session: RookerySessionResponse,
}

pub type RookeryState = Arc<Mutex<RookeryStore>>;

pub async fn post_sessions(
    Extension(store): Extension<RookeryState>,
    body: Option<Json<CreateSessionBody>>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let body = body.map(|j| j.0).unwrap_or_default();
    let mut guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let session = guard.create_session();
    let session_id = session.id.clone();
    let conversation_id = session.conversation_id.clone();

    let messages = history_to_messages(&body.history);
    if !messages.is_empty() {
        guard
            .agent
            .replace_conversation_messages(&conversation_id, messages)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    if let Some(session) = guard.get_mut(&session_id) {
        apply_session_restore(session, &body);
        RookeryStore::touch(session);
    }
    guard.persist(&session_id);

    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;
    Ok(Json(CreateSessionResponse {
        session: RookerySessionResponse::from_session(session, &output_root),
    }))
}

#[derive(Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<RookerySessionResponse>,
}

/// List all sessions (server-owned), newest first. Lets the UI render from the server.
pub async fn list_sessions(
    Extension(store): Extension<RookeryState>,
) -> Json<ListSessionsResponse> {
    let guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let mut sessions: Vec<RookerySessionResponse> = guard
        .sessions
        .values()
        .map(|s| RookerySessionResponse::from_session(s, &output_root))
        .collect();
    sessions.sort_by_key(|s| std::cmp::Reverse(s.updated_at_ms));
    Json(ListSessionsResponse { sessions })
}

pub async fn get_session(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<RookerySessionResponse>, (StatusCode, String)> {
    let guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let session = guard
        .get(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?;
    Ok(Json(RookerySessionResponse::from_session(
        session,
        &output_root,
    )))
}

pub async fn delete_session(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut guard = store.lock().await;
    guard
        .remove(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?;
    Ok(Json(json!({ "ok": true, "session_id": session_id })))
}

pub async fn post_chat(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
    Json(body): Json<RookeryChatBody>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if body.stream {
        return Ok(
            post_chat_stream(Extension(store), AxumPath(session_id), Json(body))
                .await
                .into_response(),
        );
    }
    let msg = body.message.trim();
    if msg.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "message is required".into()));
    }
    let mut guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let conv_id = guard
        .get(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?
        .conversation_id
        .clone();

    /* ops channel: apply the turn's delta batch to the session draft */
    let phase = session_ops_phase(&mut guard, &session_id, msg)
        .await
        .map_err(internal_err)?;

    /* reply channel: plain prose, informed by an ephemeral note about what was applied */
    let (model, mut messages, llm) = guard
        .agent
        .prepare_stream_turn_with_options(&conv_id, msg, false)
        .await
        .map_err(internal_err)?;
    messages.push(Message::text("system", &phase.note));
    let reply = llm.chat(&model, &messages).await.map_err(internal_err)?;
    guard.agent.add_message(&conv_id, "assistant", &reply).await;
    if let Some(ref mut s) = guard.get_mut(&session_id) {
        RookeryStore::touch(s);
    }
    guard.persist(&session_id);
    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;
    Ok(Json(RookeryChatResponse {
        reply,
        session: RookerySessionResponse::from_session(session, &output_root),
        ops_applied: phase.outcome.applied,
        ops_error: phase.outcome.error.map(|e| format!("op {}: {}", e.index, e.message)),
    })
    .into_response())
}

pub async fn post_chat_stream(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
    Json(body): Json<RookeryChatBody>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(256);
    let msg = body.message.trim().to_string();
    if msg.is_empty() {
        let payload = json!({ "type": "error", "message": "message is required" });
        let _ = tx.send(Ok(Event::default().data(payload.to_string()))).await;
        let _ = tx.send(Ok(Event::default().data(r#"{"type":"done"}"#))).await;
        return Sse::new(ReceiverStream::new(rx));
    }
    tokio::spawn(async move {
        let (prep, ops_event, note) = {
            let mut guard = store.lock().await;
            let conv_id = match guard.get(&session_id) {
                Some(s) => s.conversation_id.clone(),
                None => {
                    send_error(&tx, "session not found").await;
                    return;
                }
            };

            /* ops channel: apply the turn's delta batch to the session draft */
            let phase = match session_ops_phase(&mut guard, &session_id, &msg).await {
                Ok(p) => p,
                Err(e) => {
                    send_error(&tx, &e.to_string()).await;
                    return;
                }
            };
            let ops_event = json!({
                "type": "ops",
                "applied": phase.outcome.applied,
                "error": phase.outcome.error.as_ref().map(|e| format!("op {}: {}", e.index, e.message)),
            });

            let prep = guard
                .agent
                .prepare_stream_turn_with_options(&conv_id, &msg, false)
                .await;
            (prep, ops_event, phase.note)
        };
        let (model, mut messages, llm) = match prep {
            Ok(x) => x,
            Err(e) => {
                send_error(&tx, &e.to_string()).await;
                return;
            }
        };
        /* reply channel: plain prose, informed by an ephemeral note about what was applied */
        messages.push(Message::text("system", &note));
        if tx
            .send(Ok(Event::default().data(ops_event.to_string())))
            .await
            .is_err()
        {
            return;
        }
        let start = json!({ "type": "start", "session_id": session_id, "model": model });
        if tx
            .send(Ok(Event::default().data(start.to_string())))
            .await
            .is_err()
        {
            return;
        }
        let mut full = String::new();
        let mut stream = llm.chat_stream(&model, messages);
        while let Some(item) = stream.next().await {
            match item {
                Ok(delta) => {
                    if !delta.is_empty() {
                        full.push_str(&delta);
                        let payload = json!({ "type": "token", "content": delta });
                        if tx
                            .send(Ok(Event::default().data(payload.to_string())))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(e) => {
                    send_error(&tx, &e.to_string()).await;
                    return;
                }
            }
        }
        {
            let mut guard = store.lock().await;
            if let Some(session) = guard.get(&session_id) {
                let conv = session.conversation_id.clone();
                guard.agent.add_message(&conv, "assistant", &full).await;
                if let Some(s) = guard.get_mut(&session_id) {
                    RookeryStore::touch(s);
                }
            }
            guard.persist(&session_id);
        }
        let summary = json!({ "type": "assistant", "content": full });
        let _ = tx
            .send(Ok(Event::default().data(summary.to_string())))
            .await;
        let _ = tx
            .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
            .await;
    });
    Sse::new(ReceiverStream::new(rx))
}

async fn send_error(tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>, message: &str) {
    let payload = json!({ "type": "error", "message": message });
    let _ = tx
        .send(Ok(Event::default().data(payload.to_string())))
        .await;
    let _ = tx
        .send(Ok(Event::default().data(r#"{"type":"done"}"#)))
        .await;
}

/// [`OpsModel`] over the store's LLM provider: constrained `chat_with_schema` when the turn
/// carries a schema, plain `chat` (fenced-JSON fallback) otherwise.
struct ProviderOpsModel {
    llm: Arc<dyn LLMProvider>,
    model: String,
}

#[async_trait::async_trait]
impl OpsModel for ProviderOpsModel {
    async fn generate_ops(
        &mut self,
        messages: Vec<Message>,
        schema: Option<&serde_json::Value>,
    ) -> Result<String, KowalskiError> {
        match schema {
            Some(schema) => self.llm.chat_with_schema(&self.model, &messages, schema).await,
            None => self.llm.chat(&self.model, &messages).await,
        }
    }
}

/// Server-generated draft id for a fresh session (the draft id is server-owned; it is
/// re-slugged from `display_name` until birth).
fn draft_id_for_session(session_id: &str) -> String {
    let suffix: String = session_id
        .strip_prefix("rookery-")
        .unwrap_or(session_id)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    if suffix.is_empty() {
        "new-horde".to_string()
    } else {
        format!("horde-{suffix}")
    }
}

/// Run the ops phase for one interview turn against a session and fold the result into it:
/// apply the delta batch to the session draft (prefix-apply), re-slug the server-owned draft
/// id from `display_name` while unborn, and derive the session status. Returns the phase (its
/// `note` feeds the reply channel). The caller persists the session after the reply.
async fn session_ops_phase(
    guard: &mut RookeryStore,
    session_id: &str,
    user_message: &str,
) -> Result<OpsPhase, KowalskiError> {
    let (conv_id, draft, had_draft, born) = {
        let session = guard
            .get(session_id)
            .ok_or_else(|| KowalskiError::Validation("session not found".into()))?;
        (
            session.conversation_id.clone(),
            session
                .draft
                .clone()
                .unwrap_or_else(|| RookeryDraft::empty_draft(draft_id_for_session(&session.id))),
            session.draft.is_some(),
            session.horde_root.is_some(),
        )
    };
    let transcript = guard
        .agent
        .get_conversation(&conv_id)
        .map(|c| c.messages.clone())
        .unwrap_or_default();

    let llm = guard.agent.base().llm_provider.clone();
    let model = guard.model.clone();
    let mut config = guard.interview;
    config.structured_output =
        config.structured_output && llm.supports_structured_output(&model);
    let mut ops_model = ProviderOpsModel { llm, model };

    let mut phase = run_ops_phase(&mut ops_model, &config, &transcript, user_message, draft).await?;

    if !born
        && !phase.draft.display_name.trim().is_empty()
        && phase.outcome.applied > 0
    {
        let slug = slugify_horde_id(&phase.draft.display_name);
        if !slug.is_empty() {
            phase.draft.id = slug;
        }
    }
    if let Some(session) = guard.get_mut(session_id) {
        if had_draft || phase.outcome.applied > 0 {
            session.status = if validate_draft(&phase.draft).is_ok() {
                RookerySessionStatus::Proposed
            } else {
                RookerySessionStatus::Interviewing
            };
            session.draft = Some(phase.draft.clone());
        }
        RookeryStore::touch(session);
    }
    Ok(phase)
}

pub async fn post_propose(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<ProposeResponse>, (StatusCode, String)> {
    let mut guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let conv_id = guard
        .get(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?
        .conversation_id
        .clone();
    let reply = guard
        .agent
        .base_mut()
        .chat_with_history_with_options(&conv_id, PROPOSE_USER_MESSAGE, None, false)
        .await
        .map_err(internal_err)?;

    let mut parse_error = None;
    let draft_result = parse_draft_from_assistant(&reply);
    let summary = summarize_proposal(&reply);

    if let Some(session) = guard.get_mut(&session_id) {
        match draft_result {
            Ok(mut draft) => {
                normalize_draft(&mut draft);
                if let Err(e) = validate_draft(&draft) {
                    parse_error = Some(e.to_string());
                    session.status = RookerySessionStatus::Interviewing;
                } else {
                    session.draft = Some(draft);
                    session.status = RookerySessionStatus::Proposed;
                    session.summary = Some(summary);
                }
            }
            Err(e) => {
                parse_error = Some(e.to_string());
                session.summary = Some(summary);
            }
        }
        RookeryStore::touch(session);
    }
    guard.persist(&session_id);

    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;
    Ok(Json(ProposeResponse {
        session: RookerySessionResponse::from_session(session, &output_root),
        parse_error,
    }))
}

pub async fn post_give_birth(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
    Json(body): Json<GiveBirthBody>,
) -> Result<Json<GiveBirthResponse>, (StatusCode, String)> {
    let draft = {
        let guard = store.lock().await;
        guard
            .get(&session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?
            .draft
            .clone()
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "no draft on session; call POST .../propose first".into(),
                )
            })?
    };

    let output_root = {
        let guard = store.lock().await;
        body.output_root
            .as_ref()
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| guard.output_root.clone())
    };

    let horde_root = write_horde_tree(
        &output_root,
        &HordeBirthSpec::new(draft.clone()).with_overwrite(body.overwrite),
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let validate_result = validate_horde_tree(&horde_root);
    let (validate_ok, validate_errors) = match validate_result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };

    let mut guard = store.lock().await;
    let out_display = guard.output_root.clone();
    if let Some(session) = guard.get_mut(&session_id) {
        session.status = RookerySessionStatus::Born;
        session.horde_root = Some(horde_root.clone());
        RookeryStore::touch(session);
    }
    guard.persist(&session_id);
    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;

    Ok(Json(GiveBirthResponse {
        ok: validate_ok,
        horde_id: draft.id.clone(),
        horde_root: horde_root.display().to_string(),
        validate_ok,
        validate_errors,
        session: RookerySessionResponse::from_session(session, &out_display),
    }))
}

pub async fn patch_penguin(
    Extension(store): Extension<RookeryState>,
    AxumPath((session_id, penguin_name)): AxumPath<(String, String)>,
    Json(body): Json<PatchPenguinBody>,
) -> Result<Json<PatchPenguinResponse>, (StatusCode, String)> {
    let mut guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let session = guard
        .get_mut(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?;
    let draft = session.draft.as_mut().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "no draft on session; call POST .../propose first".into(),
        )
    })?;
    let penguin = draft
        .penguins
        .iter_mut()
        .find(|p| p.name == penguin_name)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("penguin `{penguin_name}` not found in draft"),
            )
        })?;

    if let Some(v) = body.kind {
        penguin.kind = v;
    }
    if let Some(v) = body.display_name {
        penguin.display_name = v;
    }
    if let Some(v) = body.description {
        penguin.description = v;
    }
    if let Some(v) = body.prompt_body {
        penguin.prompt_body = v;
    }
    if body.clear_agent_body {
        penguin.agent_body = None;
    } else if let Some(v) = body.agent_body {
        penguin.agent_body = Some(v);
    }
    if let Some(v) = body.output {
        penguin.output = v;
    }
    if let Some(v) = body.context_paths {
        penguin.context_paths = v;
    }
    if let Some(v) = body.tool_ids {
        penguin.tool_ids = v;
    }
    if body.clear_model_id {
        penguin.model_id = None;
    } else if let Some(v) = body.model_id {
        penguin.model_id = Some(v);
    }
    if body.clear_avatar {
        penguin.avatar = None;
    } else if let Some(v) = body.avatar {
        penguin.avatar = Some(v);
    }
    if penguin.avatar.as_deref().unwrap_or("").trim().is_empty() {
        penguin.avatar = Some(kowalski_core::infer_penguin_avatar(
            &penguin.kind,
            &penguin.name,
        ));
    }

    validate_draft(draft).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    RookeryStore::touch(session);
    guard.persist(&session_id);

    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;
    Ok(Json(PatchPenguinResponse {
        session: RookerySessionResponse::from_session(session, &output_root),
    }))
}

/// Re-write the on-disk horde from the current draft (after edits). Does not change session status.
pub async fn post_save_horde(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<SaveHordeResponse>, (StatusCode, String)> {
    let (draft, output_root) = {
        let guard = store.lock().await;
        let session = guard
            .get(&session_id)
            .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?;
        let draft = session.draft.clone().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "no draft on session; call POST .../propose first".into(),
            )
        })?;
        if session.status != RookerySessionStatus::Born {
            return Err((
                StatusCode::BAD_REQUEST,
                "session is not born; use give-birth first".into(),
            ));
        }
        let output_root = session
            .horde_root
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| guard.output_root.clone());
        (draft, output_root)
    };

    let horde_root = write_horde_tree(
        &output_root,
        &HordeBirthSpec::new(draft.clone()).with_overwrite(true),
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let validate_result = validate_horde_tree(&horde_root);
    let (validate_ok, validate_errors) = match validate_result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };

    let mut guard = store.lock().await;
    let out_display = guard.output_root.clone();
    if let Some(session) = guard.get_mut(&session_id) {
        session.horde_root = Some(horde_root.clone());
        RookeryStore::touch(session);
    }
    guard.persist(&session_id);
    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;

    Ok(Json(SaveHordeResponse {
        ok: validate_ok,
        horde_id: draft.id.clone(),
        horde_root: horde_root.display().to_string(),
        validate_ok,
        validate_errors,
        session: RookerySessionResponse::from_session(session, &out_display),
    }))
}

#[derive(Serialize)]
pub struct ValidateDraftResponse {
    pub ok: bool,
    pub errors: Option<String>,
    pub session: RookerySessionResponse,
}

pub async fn post_validate_draft(
    Extension(store): Extension<RookeryState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<ValidateDraftResponse>, (StatusCode, String)> {
    let guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let session = guard
        .get(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?;
    let draft = session.draft.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "no draft on session; call POST .../propose first".into(),
        )
    })?;
    match validate_draft(draft) {
        Ok(()) => Ok(Json(ValidateDraftResponse {
            ok: true,
            errors: None,
            session: RookerySessionResponse::from_session(session, &output_root),
        })),
        Err(e) => Ok(Json(ValidateDraftResponse {
            ok: false,
            errors: Some(e.to_string()),
            session: RookerySessionResponse::from_session(session, &output_root),
        })),
    }
}

fn summarize_proposal(text: &str) -> String {
    if let Some(json_start) = text.find("```") {
        return text[..json_start].trim().to_string();
    }
    if text.len() > 2000 {
        format!("{}…", &text[..2000])
    } else {
        text.to_string()
    }
}

fn internal_err(e: kowalski_core::KowalskiError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

fn internal_err_str(s: &str) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, s.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_output_root_is_examples_under_repo() {
        let p = default_rookery_output_root(Some(Path::new("/tmp/kowalski-test")));
        assert!(p.ends_with("examples") || p.to_string_lossy().contains("examples"));
    }

    #[test]
    fn interview_config_defaults_and_overrides() {
        let cfg = interview_config_from(&Config::default());
        assert_eq!(cfg.max_ops, InterviewConfig::DEFAULT_MAX_OPS);
        assert!(cfg.structured_output);
        assert!(!cfg.allow_replace_draft);

        let mut config = Config::default();
        config.additional.insert(
            "rookery".to_string(),
            json!({ "max_ops_per_turn": 0, "structured_output": false, "allow_replace_draft": true }),
        );
        let cfg = interview_config_from(&config);
        assert_eq!(cfg.max_ops, 1, "cap clamps to >= 1");
        assert!(!cfg.structured_output);
        assert!(cfg.allow_replace_draft);
    }

    #[test]
    fn draft_id_for_session_is_a_valid_horde_id() {
        let id = draft_id_for_session("rookery-1a2b3c4d-e5f6-7890-abcd-ef0123456789");
        assert_eq!(id, "horde-1a2b3c4d");
        kowalski_core::rookery::validate_horde_id(&id).expect("valid horde id");
        assert_eq!(draft_id_for_session("---"), "new-horde");
    }

    #[test]
    fn builder_prompt_falls_back_to_the_embedded_copy() {
        let prompt = load_builder_prompt(Path::new("/nonexistent/kowalski/config.toml"))
            .expect("embedded builder prompt");
        assert!(prompt.contains("Rookery"));
    }

    #[test]
    fn state_dir_env_override_wins() {
        let key = "KOWALSKI_ROOKERY_STATE";
        let prev = std::env::var(key).ok();
        unsafe { std::env::set_var(key, "/tmp/rookery-state-test") };
        let dir = default_rookery_state_dir(Some(Path::new("/tmp/kowalski-test")));
        assert_eq!(dir, PathBuf::from("/tmp/rookery-state-test"));
        match prev {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
    }

    #[test]
    fn state_dir_defaults_under_config_dir() {
        let key = "KOWALSKI_ROOKERY_STATE";
        let prev = std::env::var(key).ok();
        unsafe { std::env::remove_var(key) };
        let dir = default_rookery_state_dir(Some(Path::new("/tmp/kowalski-test")));
        assert!(dir.ends_with("db/rookery"));
        if let Some(v) = prev {
            unsafe { std::env::set_var(key, v) };
        }
    }

    #[test]
    fn session_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let persisted = PersistedSession {
            id: "rookery-test-1".into(),
            status: RookerySessionStatus::Proposed,
            draft: None,
            summary: Some("a summary".into()),
            horde_root: None,
            created_at_ms: 1,
            updated_at_ms: 2,
            transcript: vec![
                PersistedTurn {
                    role: "user".into(),
                    content: "build me a horde".into(),
                },
                PersistedTurn {
                    role: "assistant".into(),
                    content: "sure, what steps?".into(),
                },
            ],
        };
        write_session_file(dir.path(), &persisted).unwrap();

        let path = session_file_path(dir.path(), &persisted.id);
        assert!(path.is_file());
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("yaml"));
        let loaded: PersistedSession =
            serde_yaml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.id, "rookery-test-1");
        assert_eq!(loaded.status, RookerySessionStatus::Proposed);
        assert_eq!(loaded.summary.as_deref(), Some("a summary"));
        assert_eq!(loaded.transcript.len(), 2);
        assert_eq!(loaded.transcript[0].role, "user");
    }
}
