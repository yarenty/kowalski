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
use kowalski_core::rookery::{
    parse_draft_from_assistant, validate_draft, validate_horde_tree, write_horde_tree,
    HordeBirthSpec, RookeryDraft,
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

const BUILDER_PROMPT_REL: &str = "resources/prompts/rookery/builder.md";
const PROPOSE_USER_MESSAGE: &str = "Based on our conversation so far, emit ONLY a single ```json code block containing a complete RookeryDraft object (fields: id, display_name, description, pipeline, penguins with name, kind, display_name, description, prompt_body, output, and optional context_paths). Use a linear pipeline. No other prose.";

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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
        self.sessions.remove(id)
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
        self.sessions.insert(id, session.clone());
        session
    }

    fn touch(session: &mut RookerySession) {
        session.updated_at_ms = now_ms();
    }
}

pub async fn new_rookery_store(
    config: &Config,
    config_path: &Path,
) -> Result<Arc<Mutex<RookeryStore>>, Box<dyn std::error::Error>> {
    let prompt = load_builder_prompt(config_path)?;
    let mut agent = TemplateAgent::new(config.clone()).await?;
    agent = agent.with_system_prompt(&prompt);
    let model = config.ollama.model.clone();
    let output_root = default_rookery_output_root(config_path.parent());
    Ok(Arc::new(Mutex::new(RookeryStore {
        agent,
        model,
        output_root,
        sessions: HashMap::new(),
    })))
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

/// Load builder system prompt from repo `resources/` (several search roots).
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
    candidates.push(PathBuf::from("/opt/ml/kowalski").join(BUILDER_PROMPT_REL));

    let tried = candidates
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    for p in &candidates {
        if p.is_file() {
            return Ok(std::fs::read_to_string(p)?);
        }
    }
    Err(format!(
        "rookery builder prompt not found (tried {tried}); expected {BUILDER_PROMPT_REL}"
    )
    .into())
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

pub type RookeryState = Arc<Mutex<RookeryStore>>;

pub async fn post_sessions(
    Extension(store): Extension<RookeryState>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let mut guard = store.lock().await;
    let output_root = guard.output_root.clone();
    let session = guard.create_session();
    Ok(Json(CreateSessionResponse {
        session: RookerySessionResponse::from_session(&session, &output_root),
    }))
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
    let session = guard
        .get_mut(&session_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "session not found".into()))?;
    let conv_id = session.conversation_id.clone();
    let reply = guard
        .agent
        .base_mut()
        .chat_with_history_with_options(&conv_id, msg, None, false)
        .await
        .map_err(internal_err)?;
    if let Some(ref mut s) = guard.get_mut(&session_id) {
        RookeryStore::touch(s);
    }
    let session = guard
        .get(&session_id)
        .ok_or_else(|| internal_err_str("session disappeared"))?;
    Ok(Json(RookeryChatResponse {
        reply,
        session: RookerySessionResponse::from_session(session, &output_root),
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
        let _ = tx.send(Ok(Event::default().data(payload.to_string())));
        let _ = tx.send(Ok(Event::default().data(r#"{"type":"done"}"#)));
        return Sse::new(ReceiverStream::new(rx));
    }
    tokio::spawn(async move {
        let prep = {
            let mut guard = store.lock().await;
            let session = match guard.get(&session_id) {
                Some(s) => s.conversation_id.clone(),
                None => {
                    send_error(&tx, "session not found").await;
                    return;
                }
            };
            guard
                .agent
                .prepare_stream_turn_with_options(&session, &msg, false)
                .await
        };
        let (model, messages, llm) = match prep {
            Ok(x) => x,
            Err(e) => {
                send_error(&tx, &e.to_string()).await;
                return;
            }
        };
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
            Ok(draft) => {
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
        let p = default_rookery_output_root(Some(Path::new("/opt/ml/kowalski")));
        assert!(p.ends_with("examples") || p.to_string_lossy().contains("examples"));
    }

    #[test]
    fn load_builder_prompt_from_repo() {
        let cfg = Path::new("/opt/ml/kowalski/config.toml");
        if cfg.exists() {
            let prompt = load_builder_prompt(cfg).expect("builder.md");
            assert!(prompt.contains("Rookery"));
        }
    }
}
