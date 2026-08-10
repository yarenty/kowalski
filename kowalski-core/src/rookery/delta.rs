//! Builder delta ops — small, typed edit commands applied server-side to a [`RookeryDraft`].
//!
//! Whole-document draft emission is exactly what breaks on small local models; instead the
//! builder LLM emits one [`DeltaBatch`] (`{"ops": [...]}`) per turn. The server applies the ops
//! **in order**, validating the draft at [`DraftStrictness::Draft`] after each; on the first
//! failure it **prefix-applies** — the valid prefix is kept, the failing op and everything after
//! it are discarded, and the failure is fed back into the next interview turn.
//!
//! Prose (step prompts, descriptions) always travels as a plain-text field of an op, never as
//! JSON-in-JSON escaped by the model — that sidesteps the small-model multiline-escaping failure
//! mode that motivated this design.
//!
//! The batch contract is the embedded JSON Schema asset
//! (`resources/schemas/rookery-delta.schema.json`), kept inside a conservative JSON Schema
//! subset that constrained-decoding backends support ([`unsupported_schema_feature`]).
//! [`build_delta_schema`] shapes the per-turn variant (ops cap, `replace_draft` opt-in).
//!
//! The draft `id` is server-owned: no op edits it, and `replace_draft` preserves it.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::KowalskiError;
use crate::horde_graph::HordeEdge;
use crate::rookery::avatars::infer_penguin_avatar;
use crate::rookery::normalize::default_output_for_penguin;
use crate::rookery::types::{PenguinSpec, RookeryDraft};
use crate::rookery::validate::{validate_draft_with, validate_step_name, DraftStrictness};

/// One builder edit command. Internally tagged by `op` (matches the embedded delta schema).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum DeltaOp {
    /// Merge draft metadata (the draft `id` is server-owned, never here).
    SetMeta {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capability_prefix: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_question: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        default_topic: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workdir: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delivery_title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delivery_note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delivery_root_rel: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delivery_summary_note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt_tip: Option<String>,
    },
    /// Append a step (or insert at `index` in the pipeline). Prompt body starts empty
    /// (filled by a follow-up `set_prompt`); output gets the kind/position default.
    AddStep {
        step_id: String,
        kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Merge scalar fields of an existing step. `context_paths` replaces the whole list.
    UpdateStep {
        step_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        avatar: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_paths: Option<Vec<String>>,
    },
    /// Replace the step's prompt body wholesale.
    SetPrompt { step_id: String, prompt: String },
    /// Merge tool ids into the step's bindings (deduplicated, order preserved).
    BindTool { step_id: String, tool_ids: Vec<String> },
    /// Remove the listed tool ids; `tool_ids` omitted = remove all bindings from the step.
    UnbindTool {
        step_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_ids: Option<Vec<String>>,
    },
    /// Remove a step and its pipeline/edges references.
    RemoveStep { step_id: String },
    /// Replace the pipeline order (must list every step id exactly once).
    Reorder { pipeline: Vec<String> },
    /// Replace the draft's DAG edges. Empty = implicit linear chain along `pipeline`.
    SetEdges { edges: Vec<HordeEdge> },
    /// Replace the entire draft (whole-document escape hatch for capable models).
    /// The draft `id` is server-owned and preserved.
    ReplaceDraft { draft: Value },
}

impl DeltaOp {
    /// The op discriminator string (`set_meta`, `add_step`, …). Handy for logs / telemetry.
    pub fn kind(&self) -> &'static str {
        match self {
            DeltaOp::SetMeta { .. } => "set_meta",
            DeltaOp::AddStep { .. } => "add_step",
            DeltaOp::UpdateStep { .. } => "update_step",
            DeltaOp::SetPrompt { .. } => "set_prompt",
            DeltaOp::BindTool { .. } => "bind_tool",
            DeltaOp::UnbindTool { .. } => "unbind_tool",
            DeltaOp::RemoveStep { .. } => "remove_step",
            DeltaOp::Reorder { .. } => "reorder",
            DeltaOp::SetEdges { .. } => "set_edges",
            DeltaOp::ReplaceDraft { .. } => "replace_draft",
        }
    }
}

/// One builder turn's batch of edit commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaBatch {
    pub ops: Vec<DeltaOp>,
}

impl DeltaBatch {
    /// Parse a batch from raw JSON (the LLM's structured output or extracted fenced JSON).
    pub fn from_value(value: &Value) -> Result<Self, KowalskiError> {
        serde_json::from_value(value.clone()).map_err(|e| {
            KowalskiError::Validation(format!("delta batch is not well-formed: {e}"))
        })
    }
}

/// The failing op of a batch, with its zero-based index and the reason (fed back to the LLM).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchError {
    pub index: usize,
    pub message: String,
}

/// Result of applying a batch with prefix-apply semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchOutcome {
    /// Number of ops successfully applied (the valid prefix length).
    pub applied: usize,
    /// The first failing op, if any (ops after it were discarded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<BatchError>,
}

impl BatchOutcome {
    pub fn is_complete(&self) -> bool {
        self.error.is_none()
    }
}

/// Apply a batch to `draft` in order, validating at [`DraftStrictness::Draft`] after each op.
/// On the first failure the draft is rolled back to the state after the last good op
/// (prefix-apply) and the failure is returned.
pub fn apply_batch(draft: &mut RookeryDraft, batch: &DeltaBatch) -> BatchOutcome {
    for (index, op) in batch.ops.iter().enumerate() {
        let snapshot = draft.clone();
        let result = apply_op(draft, op)
            .and_then(|_| validate_draft_with(draft, DraftStrictness::Draft));
        if let Err(e) = result {
            *draft = snapshot;
            return BatchOutcome {
                applied: index,
                error: Some(BatchError {
                    index,
                    message: e.to_string(),
                }),
            };
        }
    }
    BatchOutcome {
        applied: batch.ops.len(),
        error: None,
    }
}

/// Apply a single op to the draft. Pure function: `(draft, op) -> draft'` with local
/// preconditions checked (referenced step exists, ids well-formed). Does **not** run
/// draft validation — [`apply_batch`] does that after each op.
pub fn apply_op(draft: &mut RookeryDraft, op: &DeltaOp) -> Result<(), KowalskiError> {
    match op {
        DeltaOp::SetMeta {
            display_name,
            description,
            capability_prefix,
            default_question,
            default_topic,
            workdir,
            delivery_title,
            delivery_note,
            delivery_root_rel,
            delivery_summary_note,
            prompt_tip,
        } => {
            if let Some(v) = display_name {
                draft.display_name = v.clone();
            }
            if let Some(v) = description {
                draft.description = v.clone();
            }
            merge_opt(&mut draft.capability_prefix, capability_prefix);
            merge_opt(&mut draft.default_question, default_question);
            merge_opt(&mut draft.default_topic, default_topic);
            merge_opt(&mut draft.workdir, workdir);
            merge_opt(&mut draft.delivery_title, delivery_title);
            merge_opt(&mut draft.delivery_note, delivery_note);
            merge_opt(&mut draft.delivery_root_rel, delivery_root_rel);
            merge_opt(&mut draft.delivery_summary_note, delivery_summary_note);
            merge_opt(&mut draft.prompt_tip, prompt_tip);
            Ok(())
        }

        DeltaOp::AddStep {
            step_id,
            kind,
            name,
            index,
        } => {
            validate_step_name(step_id)?;
            if draft.penguins.iter().any(|p| &p.name == step_id) {
                return Err(KowalskiError::Validation(format!(
                    "step `{step_id}` already exists"
                )));
            }
            let pos = index
                .map(|i| i.min(draft.pipeline.len()))
                .unwrap_or(draft.pipeline.len());
            let mut penguin = PenguinSpec {
                name: step_id.clone(),
                kind: kind.clone(),
                display_name: name.clone().unwrap_or_else(|| step_id.clone()),
                description: String::new(),
                prompt_body: String::new(),
                agent_body: None,
                output: String::new(),
                context_paths: Vec::new(),
                tool_ids: Vec::new(),
                model_id: None,
                inputs: Vec::new(),
                avatar: None,
            };
            penguin.output = default_output_for_penguin(
                draft.delivery_root_rel.as_deref(),
                &penguin,
                pos == 0,
                pos == draft.pipeline.len(),
            );
            penguin.avatar = Some(infer_penguin_avatar(kind, step_id));
            draft.penguins.push(penguin);
            draft.pipeline.insert(pos, step_id.clone());
            Ok(())
        }

        DeltaOp::UpdateStep {
            step_id,
            name,
            description,
            model_id,
            output,
            avatar,
            context_paths,
        } => {
            let penguin = require_penguin_mut(draft, step_id)?;
            if let Some(v) = name {
                penguin.display_name = v.clone();
            }
            if let Some(v) = description {
                penguin.description = v.clone();
            }
            merge_opt(&mut penguin.model_id, model_id);
            if let Some(v) = output {
                penguin.output = v.clone();
            }
            merge_opt(&mut penguin.avatar, avatar);
            if let Some(v) = context_paths {
                penguin.context_paths = v.clone();
            }
            Ok(())
        }

        DeltaOp::SetPrompt { step_id, prompt } => {
            require_penguin_mut(draft, step_id)?.prompt_body = prompt.clone();
            Ok(())
        }

        DeltaOp::BindTool { step_id, tool_ids } => {
            let penguin = require_penguin_mut(draft, step_id)?;
            for tool in tool_ids {
                if !penguin.tool_ids.contains(tool) {
                    penguin.tool_ids.push(tool.clone());
                }
            }
            Ok(())
        }

        DeltaOp::UnbindTool { step_id, tool_ids } => {
            let penguin = require_penguin_mut(draft, step_id)?;
            match tool_ids {
                None => penguin.tool_ids.clear(),
                Some(to_remove) => penguin.tool_ids.retain(|t| !to_remove.contains(t)),
            }
            Ok(())
        }

        DeltaOp::RemoveStep { step_id } => {
            if !draft.penguins.iter().any(|p| &p.name == step_id) {
                return Err(KowalskiError::Validation(format!(
                    "remove_step: unknown step `{step_id}`"
                )));
            }
            draft.penguins.retain(|p| &p.name != step_id);
            draft.pipeline.retain(|id| id != step_id);
            draft.edges.retain(|e| &e.from != step_id && &e.to != step_id);
            Ok(())
        }

        DeltaOp::Reorder { pipeline } => {
            let step_ids: HashSet<&str> = draft.penguins.iter().map(|p| p.name.as_str()).collect();
            let mut seen: HashSet<&str> = HashSet::new();
            for id in pipeline {
                if !step_ids.contains(id.as_str()) {
                    return Err(KowalskiError::Validation(format!(
                        "reorder references unknown step `{id}`"
                    )));
                }
                if !seen.insert(id.as_str()) {
                    return Err(KowalskiError::Validation(format!(
                        "reorder lists step `{id}` more than once"
                    )));
                }
            }
            if pipeline.len() != draft.penguins.len() {
                return Err(KowalskiError::Validation(
                    "reorder must list every step id exactly once".into(),
                ));
            }
            draft.pipeline = pipeline.clone();
            Ok(())
        }

        DeltaOp::SetEdges { edges } => {
            draft.edges = edges.clone();
            Ok(())
        }

        DeltaOp::ReplaceDraft { draft: value } => {
            let mut parsed: RookeryDraft = serde_json::from_value(value.clone()).map_err(|e| {
                KowalskiError::Validation(format!("replace_draft is not a well-formed draft: {e}"))
            })?;
            parsed.id = draft.id.clone();
            *draft = parsed;
            Ok(())
        }
    }
}

fn merge_opt(target: &mut Option<String>, incoming: &Option<String>) {
    if let Some(v) = incoming {
        *target = Some(v.clone());
    }
}

fn require_penguin_mut<'a>(
    draft: &'a mut RookeryDraft,
    step_id: &str,
) -> Result<&'a mut PenguinSpec, KowalskiError> {
    draft
        .penguins
        .iter_mut()
        .find(|p| p.name == step_id)
        .ok_or_else(|| KowalskiError::Validation(format!("unknown step `{step_id}`")))
}

/* --- embedded contract schema ----------------------------------------------------------------- */

/// The canonical `DeltaBatch` JSON Schema asset, embedded verbatim.
pub const DELTA_SCHEMA_JSON: &str =
    include_str!("../../resources/schemas/rookery-delta.schema.json");

/// The canonical delta-batch schema, parsed once.
pub fn base_delta_schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(DELTA_SCHEMA_JSON).expect("embedded delta schema must be valid JSON")
    })
}

/// Options that shape the delta schema handed to the model for one turn.
#[derive(Debug, Clone, Copy)]
pub struct DeltaSchemaOptions {
    /// Batch cap: the maximum number of ops the model may emit this turn (>= 1).
    pub max_ops: usize,
    /// Whether `replace_draft` (whole-document emission) is offered.
    pub allow_replace_draft: bool,
}

impl Default for DeltaSchemaOptions {
    fn default() -> Self {
        DeltaSchemaOptions {
            max_ops: 1,
            allow_replace_draft: false,
        }
    }
}

/// Build the per-turn schema variant: clamp the batch cap and drop `replace_draft` unless
/// opted in. The result is what a constrained-decoding request should carry.
pub fn build_delta_schema(opts: DeltaSchemaOptions) -> Value {
    let mut schema = base_delta_schema().clone();
    let max_ops = opts.max_ops.max(1);

    if let Some(ops) = schema.pointer_mut("/properties/ops") {
        ops["maxItems"] = json!(max_ops);
        /* a conversational turn may make zero edits (the user asked a question, not for a
         * change), so the per-turn variant allows an empty batch even though the stored
         * contract fixes minItems=1 */
        ops["minItems"] = json!(0);
    }

    if !opts.allow_replace_draft {
        /* remove the anyOf branch and its $def so the model is never offered whole-document
         * emission */
        if let Some(Value::Array(branches)) = schema.pointer_mut("/properties/ops/items/anyOf") {
            branches.retain(|b| b.get("$ref") != Some(&json!("#/$defs/replace_draft")));
        }
        if let Some(Value::Object(defs)) = schema.pointer_mut("/$defs") {
            defs.remove("replace_draft");
        }
    }

    schema
}

/* --- conservative-subset guard ---------------------------------------------------------------- */

/// JSON Schema string `format` values inside the conservative subset.
const ALLOWED_FORMATS: &[&str] = &[
    "email",
    "date",
    "time",
    "date-time",
    "duration",
    "ipv4",
    "ipv6",
    "hostname",
    "uuid",
    "uri",
    "uri-reference",
    "uri-template",
    "json-pointer",
    "relative-json-pointer",
];

/// Grammar-based constrained-decoding backends compile length/count keywords into bounded
/// repetition; large bounds blow the grammar up (observed: Ollama rejects `maxLength: 2000`
/// with "failed to parse grammar"). Prose fields should carry **no** length bound instead.
const MAX_REPETITION_BOUND: u64 = 1024;

/// Recursively check a schema for JSON Schema features outside the conservative subset that
/// constrained-decoding backends commonly support. Returns the first offending keyword found,
/// or `None` if the schema is safe.
pub fn unsupported_schema_feature(schema: &Value) -> Option<String> {
    match schema {
        Value::Object(map) => {
            for key in [
                "multipleOf",
                "uniqueItems",
                "contains",
                "minContains",
                "maxContains",
                "patternProperties",
                "propertyNames",
            ] {
                if map.contains_key(key) {
                    return Some(key.to_string());
                }
            }
            for key in ["minLength", "maxLength", "minItems", "maxItems"] {
                if let Some(bound) = map.get(key).and_then(Value::as_u64)
                    && bound > MAX_REPETITION_BOUND
                {
                    return Some(format!(
                        "{key}:{bound} (exceeds the bounded-repetition cap {MAX_REPETITION_BOUND}; drop the bound for prose fields)"
                    ));
                }
            }
            if let Some(Value::String(fmt)) = map.get("format")
                && !ALLOWED_FORMATS.contains(&fmt.as_str())
            {
                return Some(format!("format:{fmt}"));
            }
            map.values().find_map(unsupported_schema_feature)
        }
        Value::Array(items) => items.iter().find_map(unsupported_schema_feature),
        _ => None,
    }
}

/// Assert that a schema stays inside the conservative subset, returning an error rather than
/// letting a constrained-decoding backend reject the request opaquely.
pub fn ensure_schema_supported(schema: &Value) -> Result<(), KowalskiError> {
    match unsupported_schema_feature(schema) {
        Some(feature) => Err(KowalskiError::Validation(format!(
            "schema uses a feature outside the conservative constrained-decoding subset: '{feature}'"
        ))),
        None => Ok(()),
    }
}

/* --- tests ------------------------------------------------------------------------------------ */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horde_step::{StepHandlerRegistry, LLM_STEP_KINDS};
    use crate::rookery::fixture::minimal_dag_draft;
    use crate::rookery::validate::validate_draft;
    use serde_json::json;

    fn draft() -> RookeryDraft {
        RookeryDraft::empty_draft("support-triage")
    }

    fn add(step_id: &str, kind: &str) -> DeltaOp {
        DeltaOp::AddStep {
            step_id: step_id.into(),
            kind: kind.into(),
            name: None,
            index: None,
        }
    }

    /// A full scaffolding batch on an empty draft yields a birthable draft.
    #[test]
    fn worked_example_batch_builds_birthable_draft() {
        let batch_json = json!({
            "ops": [
                { "op": "set_meta", "display_name": "Support Triage",
                  "description": "Answers tickets." },
                { "op": "add_step", "step_id": "collect", "kind": "ingest", "name": "Collect" },
                { "op": "set_prompt", "step_id": "collect", "prompt": "Gather the ticket." },
                { "op": "add_step", "step_id": "draft-reply", "kind": "process" },
                { "op": "set_prompt", "step_id": "draft-reply", "prompt": "Reply politely." },
                { "op": "bind_tool", "step_id": "draft-reply", "tool_ids": ["web_search"] },
                { "op": "add_step", "step_id": "deliver", "kind": "deliver" },
                { "op": "set_prompt", "step_id": "deliver", "prompt": "Hand off the reply." },
                { "op": "reorder", "pipeline": ["collect", "draft-reply", "deliver"] }
            ]
        });
        let batch = DeltaBatch::from_value(&batch_json).unwrap();
        let mut draft = draft();
        let outcome = apply_batch(&mut draft, &batch);
        assert!(outcome.is_complete(), "outcome: {outcome:?}");
        assert_eq!(outcome.applied, 9);
        validate_draft(&draft).expect("scaffolded draft should be birthable");
        assert_eq!(draft.pipeline, vec!["collect", "draft-reply", "deliver"]);
        /* id is server-owned; set_meta never changed it */
        assert_eq!(draft.id, "support-triage");
        assert_eq!(draft.penguins[1].tool_ids, vec!["web_search"]);
        /* add_step defaults kicked in */
        assert_eq!(draft.penguins[0].output, "debug/raw/");
        assert!(draft.penguins[0].avatar.is_some());
    }

    /// Acceptance: a batch of 3 with a bad 2nd op applies exactly op 1 and reports index 1.
    #[test]
    fn prefix_apply_keeps_valid_prefix() {
        let batch = DeltaBatch {
            ops: vec![
                add("step-a", "process"),
                DeltaOp::SetPrompt {
                    step_id: "ghost".into(),
                    prompt: "nope".into(),
                },
                add("step-b", "process"),
            ],
        };
        let mut draft = draft();
        let outcome = apply_batch(&mut draft, &batch);
        assert_eq!(outcome.applied, 1);
        let err = outcome.error.expect("should fail on op 1");
        assert_eq!(err.index, 1);
        assert!(err.message.contains("ghost"));
        /* op 2 ("step-b") was discarded, op 0 survived */
        assert_eq!(draft.pipeline, vec!["step-a"]);
        assert_eq!(draft.penguins.len(), 1);
    }

    #[test]
    fn add_step_at_index_inserts_into_pipeline() {
        let mut draft = draft();
        apply_op(&mut draft, &add("step-a", "process")).unwrap();
        apply_op(
            &mut draft,
            &DeltaOp::AddStep {
                step_id: "step-b".into(),
                kind: "process".into(),
                name: Some("Step B".into()),
                index: Some(0),
            },
        )
        .unwrap();
        assert_eq!(draft.pipeline, vec!["step-b", "step-a"]);
        assert_eq!(draft.penguins[1].display_name, "Step B");
    }

    #[test]
    fn add_step_rejects_duplicates_and_bad_ids() {
        let mut draft = draft();
        let op = add("step-a", "process");
        apply_op(&mut draft, &op).unwrap();
        assert!(apply_op(&mut draft, &op).is_err());
        assert!(apply_op(&mut draft, &add("Bad/Id", "process")).is_err());
    }

    #[test]
    fn update_step_merges_fields() {
        let mut draft = draft();
        apply_op(&mut draft, &add("step-a", "process")).unwrap();
        apply_op(
            &mut draft,
            &DeltaOp::UpdateStep {
                step_id: "step-a".into(),
                name: Some("Renamed".into()),
                description: Some("Does things.".into()),
                model_id: Some("llama3.2".into()),
                output: Some("debug/custom.md".into()),
                avatar: None,
                context_paths: Some(vec!["@artifact@".into()]),
            },
        )
        .unwrap();
        let p = &draft.penguins[0];
        assert_eq!(p.display_name, "Renamed");
        assert_eq!(p.description, "Does things.");
        assert_eq!(p.model_id.as_deref(), Some("llama3.2"));
        assert_eq!(p.output, "debug/custom.md");
        assert_eq!(p.context_paths, vec!["@artifact@"]);
        /* untouched field kept its add_step default */
        assert!(p.avatar.is_some());

        let err = apply_op(
            &mut draft,
            &DeltaOp::UpdateStep {
                step_id: "ghost".into(),
                name: None,
                description: None,
                model_id: None,
                output: None,
                avatar: None,
                context_paths: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    /// An op that passes apply but fails post-op validation rolls the draft back (here:
    /// update_step escaping the workdir).
    #[test]
    fn invalid_output_path_is_rolled_back() {
        let mut draft = draft();
        apply_op(&mut draft, &add("step-a", "process")).unwrap();
        let before = draft.clone();
        let outcome = apply_batch(
            &mut draft,
            &DeltaBatch {
                ops: vec![DeltaOp::UpdateStep {
                    step_id: "step-a".into(),
                    name: None,
                    description: None,
                    model_id: None,
                    output: Some("../escape.md".into()),
                    avatar: None,
                    context_paths: None,
                }],
            },
        );
        assert_eq!(outcome.applied, 0);
        assert_eq!(outcome.error.unwrap().index, 0);
        assert_eq!(draft, before);
    }

    #[test]
    fn set_prompt_replaces_body() {
        let mut draft = draft();
        apply_op(&mut draft, &add("step-a", "process")).unwrap();
        apply_op(
            &mut draft,
            &DeltaOp::SetPrompt {
                step_id: "step-a".into(),
                prompt: "First.".into(),
            },
        )
        .unwrap();
        apply_op(
            &mut draft,
            &DeltaOp::SetPrompt {
                step_id: "step-a".into(),
                prompt: "Second.".into(),
            },
        )
        .unwrap();
        assert_eq!(draft.penguins[0].prompt_body, "Second.");
    }

    #[test]
    fn bind_then_unbind_tool() {
        let mut draft = draft();
        apply_op(&mut draft, &add("step-a", "process")).unwrap();
        apply_op(
            &mut draft,
            &DeltaOp::BindTool {
                step_id: "step-a".into(),
                tool_ids: vec!["t1".into(), "t2".into()],
            },
        )
        .unwrap();
        /* merge adds t3 without duplicating t1 */
        apply_op(
            &mut draft,
            &DeltaOp::BindTool {
                step_id: "step-a".into(),
                tool_ids: vec!["t1".into(), "t3".into()],
            },
        )
        .unwrap();
        assert_eq!(draft.penguins[0].tool_ids, vec!["t1", "t2", "t3"]);

        /* removing t2 leaves t1,t3 */
        apply_op(
            &mut draft,
            &DeltaOp::UnbindTool {
                step_id: "step-a".into(),
                tool_ids: Some(vec!["t2".into()]),
            },
        )
        .unwrap();
        assert_eq!(draft.penguins[0].tool_ids, vec!["t1", "t3"]);

        /* omitted tool_ids clears everything */
        apply_op(
            &mut draft,
            &DeltaOp::UnbindTool {
                step_id: "step-a".into(),
                tool_ids: None,
            },
        )
        .unwrap();
        assert!(draft.penguins[0].tool_ids.is_empty());
    }

    #[test]
    fn remove_step_clears_pipeline_and_edges_refs() {
        let mut draft = minimal_dag_draft();
        let victim = "branch-a";
        assert!(draft.edges.iter().any(|e| e.from == victim || e.to == victim));
        apply_op(
            &mut draft,
            &DeltaOp::RemoveStep {
                step_id: victim.into(),
            },
        )
        .unwrap();
        assert!(!draft.pipeline.iter().any(|s| s == victim));
        assert!(!draft.penguins.iter().any(|p| p.name == victim));
        assert!(!draft.edges.iter().any(|e| e.from == victim || e.to == victim));
        validate_draft_with(&draft, DraftStrictness::Draft).expect("still structurally valid");

        assert!(apply_op(
            &mut draft,
            &DeltaOp::RemoveStep {
                step_id: "ghost".into()
            }
        )
        .is_err());
    }

    #[test]
    fn reorder_must_be_permutation() {
        let mut draft = draft();
        apply_op(&mut draft, &add("step-a", "process")).unwrap();
        apply_op(&mut draft, &add("step-b", "process")).unwrap();
        assert!(apply_op(
            &mut draft,
            &DeltaOp::Reorder {
                pipeline: vec!["step-a".into()]
            }
        )
        .is_err());
        assert!(apply_op(
            &mut draft,
            &DeltaOp::Reorder {
                pipeline: vec!["step-a".into(), "step-a".into()]
            }
        )
        .is_err());
        assert!(apply_op(
            &mut draft,
            &DeltaOp::Reorder {
                pipeline: vec!["step-b".into(), "ghost".into()]
            }
        )
        .is_err());
        apply_op(
            &mut draft,
            &DeltaOp::Reorder {
                pipeline: vec!["step-b".into(), "step-a".into()],
            },
        )
        .unwrap();
        assert_eq!(draft.pipeline, vec!["step-b", "step-a"]);
    }

    /// set_edges builds a valid DAG on a linear draft (fan-out + join).
    #[test]
    fn set_edges_builds_dag() {
        let mut draft = draft();
        for id in ["ingest", "branch-a", "branch-b", "join"] {
            apply_op(&mut draft, &add(id, "process")).unwrap();
        }
        let batch = DeltaBatch::from_value(&json!({
            "ops": [{ "op": "set_edges", "edges": [
                { "from": "ingest", "to": "branch-a" },
                { "from": "ingest", "to": "branch-b" },
                { "from": "branch-a", "to": "join" },
                { "from": "branch-b", "to": "join", "when": "always" }
            ]}]
        }))
        .unwrap();
        let outcome = apply_batch(&mut draft, &batch);
        assert!(outcome.is_complete(), "outcome: {outcome:?}");
        assert_eq!(draft.edges.len(), 4);

        /* clearing edges falls back to the implicit linear chain */
        let clear = DeltaBatch {
            ops: vec![DeltaOp::SetEdges { edges: Vec::new() }],
        };
        assert!(apply_batch(&mut draft, &clear).is_complete());
        assert!(draft.edges.is_empty());
    }

    /// set_edges referencing an unknown step or an un-capped loop-back fails post-op validation
    /// and rolls back — the graph rules are enforced by the existing resolver.
    #[test]
    fn set_edges_invalid_graph_is_rolled_back() {
        let mut base = draft();
        apply_op(&mut base, &add("step-a", "process")).unwrap();
        apply_op(&mut base, &add("step-b", "process")).unwrap();

        for edges in [
            json!([{ "from": "step-a", "to": "ghost" }]),
            /* loop-back without when+max_loops */
            json!([{ "from": "step-b", "to": "step-a" }]),
        ] {
            let mut draft = base.clone();
            let batch =
                DeltaBatch::from_value(&json!({ "ops": [{ "op": "set_edges", "edges": edges }] }))
                    .unwrap();
            let outcome = apply_batch(&mut draft, &batch);
            assert_eq!(outcome.applied, 0, "edges should be rejected");
            assert_eq!(draft, base, "draft must be rolled back");
        }

        /* the same loop-back with when+max_loops (plus its forward edge) is legal */
        let mut draft = base.clone();
        let batch = DeltaBatch::from_value(&json!({ "ops": [{ "op": "set_edges", "edges": [
            { "from": "step-a", "to": "step-b" },
            { "from": "step-b", "to": "step-a", "when": "fail", "max_loops": 2 }
        ]}] }))
        .unwrap();
        let outcome = apply_batch(&mut draft, &batch);
        assert!(outcome.is_complete(), "outcome: {outcome:?}");
    }

    /// reorder on a DAG draft keeps the graph valid (edges are id-based, not position-based).
    #[test]
    fn reorder_on_dag_draft_keeps_graph_valid() {
        let mut draft = minimal_dag_draft();
        let mut reversed_mid = draft.pipeline.clone();
        reversed_mid.swap(1, 2);
        let batch = DeltaBatch {
            ops: vec![DeltaOp::Reorder {
                pipeline: reversed_mid.clone(),
            }],
        };
        let outcome = apply_batch(&mut draft, &batch);
        assert!(outcome.is_complete(), "outcome: {outcome:?}");
        assert_eq!(draft.pipeline, reversed_mid);
    }

    #[test]
    fn replace_draft_preserves_server_owned_id() {
        let mut draft = draft();
        let full = json!({
            "id": "attacker-chosen",
            "display_name": "Whole doc",
            "description": "Replaced wholesale.",
            "pipeline": ["only"],
            "penguins": [{
                "name": "only", "kind": "process", "display_name": "Only",
                "description": "d", "prompt_body": "Do it.", "output": "debug/only.md"
            }]
        });
        let outcome = apply_batch(
            &mut draft,
            &DeltaBatch {
                ops: vec![DeltaOp::ReplaceDraft { draft: full }],
            },
        );
        assert!(outcome.is_complete(), "outcome: {outcome:?}");
        assert_eq!(draft.id, "support-triage");
        assert_eq!(draft.display_name, "Whole doc");
        validate_draft(&draft).expect("replacement should be birthable");

        /* malformed document is rejected and rolled back */
        let before = draft.clone();
        let outcome = apply_batch(
            &mut draft,
            &DeltaBatch {
                ops: vec![DeltaOp::ReplaceDraft {
                    draft: json!({ "id": "x" }),
                }],
            },
        );
        assert_eq!(outcome.applied, 0);
        assert_eq!(draft, before);
    }

    #[test]
    fn set_meta_merges_only_present_fields() {
        let mut draft = draft();
        let batch = DeltaBatch::from_value(&json!({ "ops": [
            { "op": "set_meta", "display_name": "Named", "workdir": "output",
              "delivery_root_rel": "HANDOFF.md" },
            { "op": "set_meta", "description": "Later." }
        ] }))
        .unwrap();
        assert!(apply_batch(&mut draft, &batch).is_complete());
        assert_eq!(draft.display_name, "Named");
        assert_eq!(draft.description, "Later.");
        assert_eq!(draft.workdir.as_deref(), Some("output"));
        assert_eq!(draft.delivery_root_rel.as_deref(), Some("HANDOFF.md"));
        assert!(draft.prompt_tip.is_none());
    }

    #[test]
    fn empty_draft_is_draft_valid_but_not_birthable() {
        let draft = draft();
        validate_draft_with(&draft, DraftStrictness::Draft).expect("empty draft is a valid start");
        assert!(validate_draft(&draft).is_err());
    }

    /// The op enum round-trips through JSON with the `op` discriminator.
    #[test]
    fn op_json_round_trip() {
        let op = DeltaOp::SetPrompt {
            step_id: "x".into(),
            prompt: "hello".into(),
        };
        let v = serde_json::to_value(&op).unwrap();
        assert_eq!(v["op"], json!("set_prompt"));
        let back: DeltaOp = serde_json::from_value(v).unwrap();
        assert_eq!(back, op);
    }

    /* --- schema asset ------------------------------------------------------------------------- */

    #[test]
    fn base_schema_parses_and_stays_in_subset() {
        let schema = base_delta_schema();
        assert_eq!(schema["title"], "DeltaBatch");
        assert_eq!(unsupported_schema_feature(schema), None);
        assert!(ensure_schema_supported(schema).is_ok());
    }

    /// Every DeltaOp variant has a schema branch and vice versa — vocabulary and contract
    /// cannot drift apart.
    #[test]
    fn schema_defs_match_op_vocabulary() {
        let ops = [
            DeltaOp::SetMeta {
                display_name: None,
                description: None,
                capability_prefix: None,
                default_question: None,
                default_topic: None,
                workdir: None,
                delivery_title: None,
                delivery_note: None,
                delivery_root_rel: None,
                delivery_summary_note: None,
                prompt_tip: None,
            },
            add("s", "process"),
            DeltaOp::UpdateStep {
                step_id: "s".into(),
                name: None,
                description: None,
                model_id: None,
                output: None,
                avatar: None,
                context_paths: None,
            },
            DeltaOp::SetPrompt {
                step_id: "s".into(),
                prompt: "p".into(),
            },
            DeltaOp::BindTool {
                step_id: "s".into(),
                tool_ids: vec!["t".into()],
            },
            DeltaOp::UnbindTool {
                step_id: "s".into(),
                tool_ids: None,
            },
            DeltaOp::RemoveStep {
                step_id: "s".into(),
            },
            DeltaOp::Reorder {
                pipeline: vec!["s".into()],
            },
            DeltaOp::SetEdges { edges: Vec::new() },
            DeltaOp::ReplaceDraft { draft: json!({}) },
        ];
        let mut kinds: Vec<&str> = ops.iter().map(|o| o.kind()).collect();
        kinds.sort_unstable();

        let branches = base_delta_schema()["properties"]["ops"]["items"]["anyOf"]
            .as_array()
            .unwrap();
        let mut refs: Vec<String> = branches
            .iter()
            .map(|b| {
                b["$ref"]
                    .as_str()
                    .unwrap()
                    .trim_start_matches("#/$defs/")
                    .to_string()
            })
            .collect();
        refs.sort_unstable();
        assert_eq!(refs, kinds);
    }

    /// The add_step kind enum tracks the kinds the runtime actually executes.
    #[test]
    fn schema_kind_enum_matches_runtime_kinds() {
        let mut runtime: Vec<String> = LLM_STEP_KINDS.iter().map(|k| k.to_string()).collect();
        runtime.extend(
            StepHandlerRegistry::with_builtin_deterministic()
                .kinds()
                .iter()
                .map(|k| k.to_string()),
        );
        runtime.sort_unstable();

        let mut schema_kinds: Vec<String> = base_delta_schema()["$defs"]["add_step"]["properties"]
            ["kind"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        schema_kinds.sort_unstable();
        assert_eq!(schema_kinds, runtime);
    }

    #[test]
    fn schema_validates_sample_batches() {
        let validator = jsonschema::validator_for(base_delta_schema()).unwrap();

        let good = json!({ "ops": [
            { "op": "set_meta", "display_name": "X" },
            { "op": "add_step", "step_id": "collect", "kind": "ingest", "index": 0 },
            { "op": "set_prompt", "step_id": "collect", "prompt": "Multi-line\nprose is fine." },
            { "op": "update_step", "step_id": "collect", "model_id": "llama3.2",
              "context_paths": ["@artifact@"] },
            { "op": "bind_tool", "step_id": "collect", "tool_ids": ["web_search"] },
            { "op": "unbind_tool", "step_id": "collect" },
            { "op": "reorder", "pipeline": ["collect"] },
            { "op": "set_edges", "edges": [
                { "from": "a", "to": "b", "when": "fail", "max_loops": 2 } ] },
            { "op": "remove_step", "step_id": "collect" },
            { "op": "replace_draft", "draft": { "id": "x" } }
        ]});
        assert!(validator.is_valid(&good), "sample batch should validate");

        for bad in [
            json!({ "ops": [] }),
            json!({ "ops": [{ "op": "explode" }] }),
            json!({ "ops": [{ "op": "add_step", "step_id": "UpperCase", "kind": "process" }] }),
            json!({ "ops": [{ "op": "add_step", "step_id": "s", "kind": "not-a-kind" }] }),
            json!({ "ops": [{ "op": "set_prompt", "step_id": "s" }] }),
            json!({ "ops": [{ "op": "set_edges", "edges": [{ "from": "a" }] }] }),
            json!({ "ops": [{ "op": "set_meta", "extra": true }] }),
        ] {
            assert!(!validator.is_valid(&bad), "should reject: {bad}");
        }
    }

    #[test]
    fn built_variants_cap_ops_and_strip_replace_draft() {
        let capped = build_delta_schema(DeltaSchemaOptions {
            max_ops: 2,
            allow_replace_draft: false,
        });
        assert_eq!(capped["properties"]["ops"]["maxItems"], 2);
        assert_eq!(capped["properties"]["ops"]["minItems"], 0);
        assert!(capped["$defs"].get("replace_draft").is_none());
        assert_eq!(unsupported_schema_feature(&capped), None);

        let validator = jsonschema::validator_for(&capped).unwrap();
        assert!(validator.is_valid(&json!({ "ops": [] })));
        assert!(!validator.is_valid(&json!({ "ops": [
            { "op": "remove_step", "step_id": "a" },
            { "op": "remove_step", "step_id": "b" },
            { "op": "remove_step", "step_id": "c" }
        ]})));
        assert!(
            !validator.is_valid(&json!({ "ops": [{ "op": "replace_draft", "draft": {} }] })),
            "stripped variant must reject replace_draft"
        );

        let full = build_delta_schema(DeltaSchemaOptions {
            max_ops: 0, /* clamped to 1 */
            allow_replace_draft: true,
        });
        assert_eq!(full["properties"]["ops"]["maxItems"], 1);
        assert!(full["$defs"].get("replace_draft").is_some());
        assert_eq!(unsupported_schema_feature(&full), None);
        let validator = jsonschema::validator_for(&full).unwrap();
        assert!(validator.is_valid(&json!({ "ops": [{ "op": "replace_draft", "draft": {} }] })));
    }

    /// The subset guard rejects a schema using features outside the conservative subset.
    #[test]
    fn subset_guard_rejects_violations() {
        for (bad, expect) in [
            (json!({ "type": "array", "uniqueItems": true }), "uniqueItems"),
            (json!({ "type": "number", "multipleOf": 2 }), "multipleOf"),
            (
                json!({ "type": "object", "properties": { "x": {
                    "type": "object", "patternProperties": { "^a": {} } } } }),
                "patternProperties",
            ),
            (
                json!({ "type": "string", "format": "email-list" }),
                "format:email-list",
            ),
            (
                json!({ "type": "string", "maxLength": 4000 }),
                "maxLength:4000 (exceeds the bounded-repetition cap 1024; drop the bound for prose fields)",
            ),
        ] {
            assert_eq!(unsupported_schema_feature(&bad), Some(expect.to_string()));
            assert!(ensure_schema_supported(&bad).is_err());
        }
        assert_eq!(
            unsupported_schema_feature(&json!({ "type": "string", "format": "uuid" })),
            None
        );
    }

    /* --- property-ish: random valid op sequences keep the draft valid -------------------------- */

    /// Deterministic LCG so the test needs no RNG dependency and failures reproduce.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, bound: usize) -> usize {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 33) % bound.max(1) as u64) as usize
        }
    }

    /// Random op sequences — including nonsense ops — never leave the draft Draft-invalid:
    /// apply_batch either applies a valid prefix or rolls back to the last good state.
    #[test]
    fn random_op_sequences_keep_draft_valid() {
        let kinds = ["process", "ingest", "deliver", "verify"];
        for seed in 0..25u64 {
            let mut rng = Lcg(seed + 1);
            let mut draft = RookeryDraft::empty_draft("prop-horde");
            for _ in 0..40 {
                let existing: Vec<String> = draft.pipeline.clone();
                let pick = |rng: &mut Lcg, existing: &[String]| -> String {
                    if existing.is_empty() || rng.next(5) == 0 {
                        "ghost".to_string() /* sometimes reference a missing step */
                    } else {
                        existing[rng.next(existing.len())].clone()
                    }
                };
                let op = match rng.next(8) {
                    0 => DeltaOp::AddStep {
                        step_id: format!("step-{}", rng.next(10)),
                        kind: kinds[rng.next(kinds.len())].into(),
                        name: None,
                        index: Some(rng.next(existing.len() + 2)),
                    },
                    1 => DeltaOp::SetPrompt {
                        step_id: pick(&mut rng, &existing),
                        prompt: "Do the thing.".into(),
                    },
                    2 => DeltaOp::UpdateStep {
                        step_id: pick(&mut rng, &existing),
                        name: Some("N".into()),
                        description: None,
                        model_id: None,
                        output: Some(format!("debug/out-{}.md", rng.next(10))),
                        avatar: None,
                        context_paths: None,
                    },
                    3 => DeltaOp::BindTool {
                        step_id: pick(&mut rng, &existing),
                        tool_ids: vec![format!("tool-{}", rng.next(3))],
                    },
                    4 => DeltaOp::UnbindTool {
                        step_id: pick(&mut rng, &existing),
                        tool_ids: None,
                    },
                    5 => DeltaOp::RemoveStep {
                        step_id: pick(&mut rng, &existing),
                    },
                    6 => {
                        let mut pipeline = existing.clone();
                        if !pipeline.is_empty() {
                            let i = rng.next(pipeline.len());
                            let j = rng.next(pipeline.len());
                            pipeline.swap(i, j);
                        }
                        DeltaOp::Reorder { pipeline }
                    }
                    _ => {
                        let mut edges = Vec::new();
                        for w in existing.windows(2) {
                            if rng.next(2) == 0 {
                                edges.push(HordeEdge {
                                    from: w[0].clone(),
                                    to: w[1].clone(),
                                    when: None,
                                    max_loops: None,
                                });
                            }
                        }
                        DeltaOp::SetEdges { edges }
                    }
                };
                apply_batch(&mut draft, &DeltaBatch { ops: vec![op] });
                validate_draft_with(&draft, DraftStrictness::Draft).unwrap_or_else(|e| {
                    panic!("seed {seed}: draft became invalid: {e}\n{draft:#?}")
                });
            }
        }
    }
}
