//! The guided-interview ops phase — the per-turn delta half of the Rookery builder split.
//!
//! Each interview turn runs two channels; this module owns the first:
//! 1. **ops** — ask the model (under a structured-output constraint, or fenced-JSON fallback)
//!    for a small [`DeltaBatch`], parse it, and apply it to the draft with prefix-apply;
//! 2. **reply** — the caller then asks for a plain-text conversational answer, informed by the
//!    [`OpsPhase::note`] this phase produces.
//!
//! The LLM is abstracted behind [`OpsModel`] so the whole phase — prompt assembly, parsing,
//! prefix-apply, replace-draft gating — is unit-testable without a live model. The HTTP server
//! drives it with a real provider (`chat_with_schema` when the deployment opts in).
//!
//! A malformed ops response never fails the turn — it degrades to "no edits this turn" so the
//! conversation continues; the phase errors only if the model call itself errors.

use async_trait::async_trait;
use serde_json::Value;

use crate::conversation::Message;
use crate::error::KowalskiError;
use crate::rookery::delta::{
    apply_batch, build_delta_schema, ensure_schema_supported, BatchError, BatchOutcome,
    DeltaBatch, DeltaOp, DeltaSchemaOptions,
};
use crate::rookery::types::RookeryDraft;
use crate::rookery::validate::{validate_draft_with, DraftStrictness};
use crate::utils::json::extract_first_json_object;

/// The ops-channel system prompt: JSON-only delta output with the op cheat-sheet. Embedded
/// beside the op vocabulary it must describe.
pub const OPS_SYSTEM_PROMPT: &str = include_str!("../../resources/prompts/rookery-delta-ops.md");

/// Per-deployment interview policy (`[rookery]` in the server config). Defaults are chosen so
/// an installation never has to tune anything.
#[derive(Debug, Clone, Copy)]
pub struct InterviewConfig {
    /// Batch cap: the maximum number of ops the model may emit per turn (>= 1).
    pub max_ops: usize,
    /// Whether the ops channel uses constrained structured output
    /// ([`crate::llm::LLMProvider::chat_with_schema`]). When false, the phase falls back to
    /// fenced-JSON extraction with the same op vocabulary.
    pub structured_output: bool,
    /// Whether whole-document emission (`replace_draft`) is offered to the model.
    pub allow_replace_draft: bool,
}

impl InterviewConfig {
    pub const DEFAULT_MAX_OPS: usize = 12;
}

impl Default for InterviewConfig {
    fn default() -> Self {
        InterviewConfig {
            max_ops: Self::DEFAULT_MAX_OPS,
            structured_output: true,
            allow_replace_draft: false,
        }
    }
}

/// The single model call the ops phase needs. Abstracted so the phase is testable without
/// inference.
#[async_trait]
pub trait OpsModel: Send {
    /// Generate the delta batch. `schema` is the per-turn delta schema when structured output
    /// is enabled, or `None` for the fenced-JSON fallback. Returns the raw model text.
    async fn generate_ops(
        &mut self,
        messages: Vec<Message>,
        schema: Option<&Value>,
    ) -> Result<String, KowalskiError>;
}

/// The outcome of one ops phase.
#[derive(Debug, Clone)]
pub struct OpsPhase {
    /// The draft after applying the turn's ops (prefix-applied on failure).
    pub draft: RookeryDraft,
    /// What the ops channel did (applied count + first failure, if any).
    pub outcome: BatchOutcome,
    /// A short note describing what was applied — inject it as an ephemeral system message
    /// into the reply channel so the conversational answer matches reality.
    pub note: String,
}

/// Run the ops phase for one interview turn: build the per-turn schema (when structured
/// output is on), ask the model for a batch, and apply it with prefix-apply semantics.
pub async fn run_ops_phase<M: OpsModel + ?Sized>(
    model: &mut M,
    config: &InterviewConfig,
    transcript: &[Message],
    user_message: &str,
    mut draft: RookeryDraft,
) -> Result<OpsPhase, KowalskiError> {
    let schema = if config.structured_output {
        let schema = build_delta_schema(DeltaSchemaOptions {
            max_ops: config.max_ops,
            allow_replace_draft: config.allow_replace_draft,
        });
        ensure_schema_supported(&schema)?;
        Some(schema)
    } else {
        None
    };

    let messages = ops_call_messages(transcript, user_message, &draft);
    let ops_text = model.generate_ops(messages, schema.as_ref()).await?;
    let outcome = apply_ops_text(&mut draft, &ops_text, config);
    log::debug!(
        "rookery interview: ops batch applied={} complete={}",
        outcome.applied,
        outcome.is_complete()
    );

    let note = turn_note(&outcome, &draft);
    Ok(OpsPhase {
        draft,
        outcome,
        note,
    })
}

/// Parse the ops text and apply it. A batch that cannot be parsed (only possible on the
/// fenced-JSON fallback path — structured outputs are valid by construction) is logged and
/// treated as an empty batch so the turn still produces a reply. `replace_draft` is cut out
/// of the batch (with an error at its index) unless the config allows it.
fn apply_ops_text(draft: &mut RookeryDraft, ops_text: &str, config: &InterviewConfig) -> BatchOutcome {
    let Some(value) = extract_first_json_object(ops_text) else {
        log::warn!("rookery interview: model output contained no JSON object; treating as no edits");
        return BatchOutcome {
            applied: 0,
            error: None,
        };
    };
    let mut batch = match DeltaBatch::from_value(&value) {
        Ok(batch) => batch,
        Err(e) => {
            log::warn!("rookery interview: could not parse delta batch ({e}); treating as no edits");
            return BatchOutcome {
                applied: 0,
                error: None,
            };
        }
    };
    if batch.ops.len() > config.max_ops {
        batch.ops.truncate(config.max_ops);
    }

    /* gate the whole-document escape hatch on the fallback path (the constrained path never
     * offers it — build_delta_schema strips it from the schema) */
    let blocked = if config.allow_replace_draft {
        None
    } else {
        batch
            .ops
            .iter()
            .position(|op| matches!(op, DeltaOp::ReplaceDraft { .. }))
    };
    if let Some(index) = blocked {
        batch.ops.truncate(index);
        let mut outcome = apply_batch(draft, &batch);
        if outcome.error.is_none() {
            outcome.error = Some(BatchError {
                index,
                message: "replace_draft is disabled on this deployment — emit incremental ops instead".to_string(),
            });
        }
        return outcome;
    }

    apply_batch(draft, &batch)
}

/// Build the message list for the **ops channel**: the ops system prompt, the prior
/// conversation (user/assistant turns only), the new user message, then a directive showing
/// the current draft and asking for the edit ops. The model's reply is constrained to a
/// delta batch.
pub fn ops_call_messages(
    transcript: &[Message],
    user_message: &str,
    draft: &RookeryDraft,
) -> Vec<Message> {
    let mut messages = Vec::with_capacity(transcript.len() + 3);
    messages.push(Message::text("system", OPS_SYSTEM_PROMPT));
    messages.extend(
        transcript
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .cloned(),
    );
    messages.push(Message::text("user", user_message));

    let draft_json = serde_json::to_string_pretty(draft).unwrap_or_default();
    let existing_ids = if draft.pipeline.is_empty() {
        "(none yet — every step must be created with add_step before it can be configured)"
            .to_string()
    } else {
        draft.pipeline.join(", ")
    };
    let directive = format!(
        "Current horde draft (JSON):\n{draft_json}\n\nExisting step ids: {existing_ids}\n\n\
         Return ONLY a single JSON object of the form {{\"ops\": [ ... ]}} containing the edit \
         operations that apply the operator's latest request to this draft — no prose, no code \
         fences. Use an empty array ({{\"ops\": []}}) if the message needs no change to the \
         draft (for example, a question). Prompts and other prose go in the dedicated text \
         fields of the ops."
    );
    messages.push(Message::text("system", &directive));
    messages
}

/// A short, human-readable note describing what the ops channel did this turn — fed to the
/// reply channel so the conversational answer matches reality, and useful in logs.
pub fn turn_note(outcome: &BatchOutcome, draft: &RookeryDraft) -> String {
    let mut note = if outcome.applied == 0 && outcome.error.is_none() {
        "You made no changes to the horde draft this turn.".to_string()
    } else {
        format!(
            "You applied {} edit operation(s) to the horde draft.",
            outcome.applied
        )
    };
    if let Some(err) = &outcome.error {
        note.push_str(&format!(
            " Operation {} was rejected and discarded (with everything after it): {}. Explain \
             briefly and, if useful, try a corrected edit next turn.",
            err.index, err.message
        ));
    }
    if draft.pipeline.is_empty() {
        note.push_str(" The draft currently has no steps.");
    } else {
        note.push_str(&format!(" The pipeline is now: {}.", draft.pipeline.join(" → ")));
    }
    if draft.display_name.trim().is_empty() {
        note.push_str(" The draft has no display name yet.");
    }
    if !draft.pipeline.is_empty() && validate_draft_with(draft, DraftStrictness::Draft).is_ok() {
        let unfinished: Vec<&str> = draft
            .penguins
            .iter()
            .filter(|p| p.prompt_body.trim().is_empty())
            .map(|p| p.name.as_str())
            .collect();
        if !unfinished.is_empty() {
            note.push_str(&format!(
                " Steps still missing a prompt: {}.",
                unfinished.join(", ")
            ));
        }
    }
    note
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rookery::validate::validate_draft;
    use crate::rookery::writer::write_horde_tree;
    use crate::rookery::{validate_horde_tree, HordeBirthSpec};
    use serde_json::json;
    use std::collections::VecDeque;

    /// A scripted model: replies are handed out in order; records the schema it was handed.
    struct ScriptedModel {
        replies: VecDeque<String>,
        last_schema: Option<Value>,
        calls: usize,
    }

    impl ScriptedModel {
        fn new(replies: Vec<String>) -> Self {
            ScriptedModel {
                replies: replies.into_iter().collect(),
                last_schema: None,
                calls: 0,
            }
        }
    }

    #[async_trait]
    impl OpsModel for ScriptedModel {
        async fn generate_ops(
            &mut self,
            _messages: Vec<Message>,
            schema: Option<&Value>,
        ) -> Result<String, KowalskiError> {
            self.calls += 1;
            self.last_schema = schema.cloned();
            Ok(self
                .replies
                .pop_front()
                .unwrap_or_else(|| r#"{"ops":[]}"#.to_string()))
        }
    }

    fn cfg() -> InterviewConfig {
        InterviewConfig::default()
    }

    fn draft() -> RookeryDraft {
        RookeryDraft::empty_draft("test-horde")
    }

    #[tokio::test]
    async fn phase_applies_ops_and_reports_note() {
        let ops = json!({ "ops": [
            { "op": "set_meta", "display_name": "Collector" },
            { "op": "add_step", "step_id": "collect", "kind": "ingest" },
            { "op": "set_prompt", "step_id": "collect", "prompt": "Gather sources." }
        ]})
        .to_string();
        let mut model = ScriptedModel::new(vec![ops]);
        let phase = run_ops_phase(&mut model, &cfg(), &[], "collect sources", draft())
            .await
            .unwrap();

        assert_eq!(phase.outcome.applied, 3);
        assert!(phase.outcome.is_complete());
        assert_eq!(phase.draft.pipeline, vec!["collect"]);
        assert!(phase.note.contains("3 edit operation(s)"));
        assert!(phase.note.contains("collect"));
        /* structured output on → the ops call was handed a schema without replace_draft */
        let schema = model.last_schema.expect("schema handed to the model");
        assert_eq!(schema["properties"]["ops"]["maxItems"], 12);
        assert!(schema["$defs"].get("replace_draft").is_none());
    }

    #[tokio::test]
    async fn phase_prefix_applies_bad_batch_and_note_reports_failure() {
        let ops = json!({ "ops": [
            { "op": "add_step", "step_id": "collect", "kind": "ingest" },
            { "op": "set_prompt", "step_id": "ghost", "prompt": "x" },
            { "op": "add_step", "step_id": "after", "kind": "process" }
        ]})
        .to_string();
        let mut model = ScriptedModel::new(vec![ops]);
        let phase = run_ops_phase(&mut model, &cfg(), &[], "do it", draft())
            .await
            .unwrap();

        assert_eq!(phase.outcome.applied, 1);
        assert_eq!(phase.outcome.error.as_ref().unwrap().index, 1);
        assert_eq!(phase.draft.pipeline, vec!["collect"]);
        assert!(phase.note.contains("Operation 1 was rejected"));
        assert!(phase.note.contains("ghost"));
    }

    #[tokio::test]
    async fn empty_batch_is_a_clean_no_edit_turn() {
        let mut model = ScriptedModel::new(vec![r#"{"ops":[]}"#.to_string()]);
        let phase = run_ops_phase(&mut model, &cfg(), &[], "just a question", draft())
            .await
            .unwrap();
        assert_eq!(phase.outcome.applied, 0);
        assert!(phase.outcome.is_complete());
        assert!(phase.note.contains("no changes"));
    }

    #[tokio::test]
    async fn garbage_ops_degrade_to_no_edits() {
        let mut model =
            ScriptedModel::new(vec!["I'm not going to give you JSON, sorry.".to_string()]);
        let before = draft();
        let phase = run_ops_phase(&mut model, &cfg(), &[], "hi", before.clone())
            .await
            .unwrap();
        assert_eq!(phase.outcome.applied, 0);
        assert!(phase.outcome.is_complete());
        assert_eq!(phase.draft, before);
    }

    #[tokio::test]
    async fn fenced_json_fallback_parses_without_schema() {
        let ops = format!(
            "Sure, here are the edits:\n```json\n{}\n```",
            json!({ "ops": [ { "op": "add_step", "step_id": "collect", "kind": "ingest" } ] })
        );
        let mut model = ScriptedModel::new(vec![ops]);
        let config = InterviewConfig {
            structured_output: false,
            ..cfg()
        };
        let phase = run_ops_phase(&mut model, &config, &[], "add collect", draft())
            .await
            .unwrap();
        assert!(model.last_schema.is_none(), "fallback sends no schema");
        assert_eq!(phase.outcome.applied, 1);
        assert_eq!(phase.draft.pipeline, vec!["collect"]);
    }

    #[tokio::test]
    async fn replace_draft_blocked_unless_allowed() {
        let ops = json!({ "ops": [
            { "op": "add_step", "step_id": "collect", "kind": "ingest" },
            { "op": "replace_draft", "draft": { "id": "x", "display_name": "X",
              "description": "", "pipeline": [], "penguins": [] } }
        ]})
        .to_string();

        let config = InterviewConfig {
            structured_output: false,
            ..cfg()
        };
        let mut model = ScriptedModel::new(vec![ops.clone()]);
        let phase = run_ops_phase(&mut model, &config, &[], "replace it", draft())
            .await
            .unwrap();
        /* the prefix before replace_draft applied; the escape hatch itself was refused */
        assert_eq!(phase.outcome.applied, 1);
        let err = phase.outcome.error.as_ref().unwrap();
        assert_eq!(err.index, 1);
        assert!(err.message.contains("replace_draft is disabled"));
        assert_eq!(phase.draft.pipeline, vec!["collect"]);

        let allowed = InterviewConfig {
            structured_output: false,
            allow_replace_draft: true,
            ..cfg()
        };
        let mut model = ScriptedModel::new(vec![ops]);
        let phase = run_ops_phase(&mut model, &allowed, &[], "replace it", draft())
            .await
            .unwrap();
        assert!(phase.outcome.is_complete());
        assert_eq!(phase.draft.display_name, "X");
        assert_eq!(phase.draft.id, "test-horde", "id stays server-owned");
    }

    #[tokio::test]
    async fn oversized_batch_is_truncated_to_max_ops() {
        let ops = json!({ "ops": [
            { "op": "add_step", "step_id": "s1", "kind": "process" },
            { "op": "add_step", "step_id": "s2", "kind": "process" },
            { "op": "add_step", "step_id": "s3", "kind": "process" }
        ]})
        .to_string();
        let config = InterviewConfig {
            max_ops: 2,
            structured_output: false,
            ..cfg()
        };
        let mut model = ScriptedModel::new(vec![ops]);
        let phase = run_ops_phase(&mut model, &config, &[], "add three", draft())
            .await
            .unwrap();
        assert_eq!(phase.outcome.applied, 2);
        assert_eq!(phase.draft.pipeline, vec!["s1", "s2"]);
    }

    #[tokio::test]
    async fn ops_messages_carry_prompt_transcript_and_draft() {
        let transcript = vec![
            Message::text("system", "builder persona (must be filtered out)"),
            Message::text("user", "I want a research horde"),
            Message::text("assistant", "Sure — what sources?"),
        ];
        let mut d = draft();
        d.display_name = "Research".into();
        let messages = ops_call_messages(&transcript, "web pages", &d);

        assert_eq!(messages[0].role, "system");
        assert!(messages[0].content.contains("edit operations"));
        /* persona system message filtered; user/assistant kept in order */
        assert_eq!(messages[1].content, "I want a research horde");
        assert_eq!(messages[2].content, "Sure — what sources?");
        assert_eq!(messages[3].content, "web pages");
        let directive = &messages[4];
        assert_eq!(directive.role, "system");
        assert!(directive.content.contains("\"display_name\": \"Research\""));
    }

    /// The offline analogue of the acceptance demo: a scripted multi-turn interview builds a
    /// birthable 3-step horde (with one edge op) purely through per-turn deltas, then the
    /// draft births to disk and the written tree validates.
    #[tokio::test]
    async fn multi_turn_interview_reaches_birthable_horde_and_births() {
        let turn1 = json!({ "ops": [
            { "op": "set_meta", "display_name": "Research Digest",
              "description": "Collects sources and writes a digest." },
            { "op": "add_step", "step_id": "collect", "kind": "ingest" },
            { "op": "set_prompt", "step_id": "collect", "prompt": "Gather the sources." }
        ]})
        .to_string();
        let turn2 = json!({ "ops": [
            { "op": "add_step", "step_id": "digest", "kind": "process" },
            { "op": "set_prompt", "step_id": "digest", "prompt": "Summarize the material." },
            { "op": "add_step", "step_id": "deliver", "kind": "deliver" },
            { "op": "set_prompt", "step_id": "deliver", "prompt": "Write the handoff." }
        ]})
        .to_string();
        let turn3 = json!({ "ops": [
            { "op": "set_edges", "edges": [
                { "from": "collect", "to": "digest" },
                { "from": "digest", "to": "deliver" },
                { "from": "deliver", "to": "digest", "when": "fail", "max_loops": 1 }
            ]}
        ]})
        .to_string();

        let mut model = ScriptedModel::new(vec![turn1, turn2, turn3]);
        let mut draft = RookeryDraft::empty_draft("research-digest");
        let mut transcript: Vec<Message> = Vec::new();

        for msg in ["collect + digest", "add digest and deliver", "wire the edges"] {
            let phase = run_ops_phase(&mut model, &cfg(), &transcript, msg, draft)
                .await
                .unwrap();
            assert!(
                phase.outcome.is_complete(),
                "turn `{msg}` failed: {:?}",
                phase.outcome
            );
            draft = phase.draft;
            transcript.push(Message::text("user", msg));
            transcript.push(Message::text("assistant", "Done."));
        }

        assert_eq!(model.calls, 3);
        assert_eq!(draft.pipeline, vec!["collect", "digest", "deliver"]);
        assert_eq!(draft.edges.len(), 3);
        validate_draft(&draft).expect("interview result should be birthable");

        let dir = tempfile::tempdir().unwrap();
        let root = write_horde_tree(dir.path(), &HordeBirthSpec::new(draft)).unwrap();
        validate_horde_tree(&root).expect("born tree should validate");
    }
}
