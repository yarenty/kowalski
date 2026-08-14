//! Rookery draft types (linear pipeline in 1.3.0; optional `edges[]` in 1.5.0+).

use crate::horde_graph::HordeEdge;
use crate::horde_trigger::HordeTrigger;
use crate::operator_input::OperatorInputField;
use serde::{Deserialize, Serialize};

/// In-memory draft between interview and **Give birth** (linear `pipeline` order only).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RookeryDraft {
    pub id: String,
    pub display_name: String,
    pub description: String,
    /// Semver workflow version (`version = "x.y.z"` in horde.md; used by manifest export).
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub capability_prefix: Option<String>,
    /// Ordered step names; must match `penguins` keys exactly.
    pub pipeline: Vec<String>,
    /// Optional DAG edges; absent or empty → implicit chain along `pipeline`.
    #[serde(default)]
    pub edges: Vec<HordeEdge>,
    /// Optional event-driven run declarations (`[[triggers]]` in horde.md).
    #[serde(default)]
    pub triggers: Vec<HordeTrigger>,
    pub penguins: Vec<PenguinSpec>,
    #[serde(default)]
    pub default_question: Option<String>,
    #[serde(default)]
    pub default_topic: Option<String>,
    /// Relative workdir under horde root (default `output`).
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub delivery_title: Option<String>,
    #[serde(default)]
    pub delivery_note: Option<String>,
    /// Relative to workdir (e.g. `HANDOFF.md`).
    #[serde(default)]
    pub delivery_root_rel: Option<String>,
    #[serde(default)]
    pub delivery_summary_note: Option<String>,
    #[serde(default)]
    pub prompt_tip: Option<String>,
}

impl RookeryDraft {
    /// A blank draft for the builder interview: only the server-owned `id` is set; everything
    /// else is filled in via delta ops. Passes `DraftStrictness::Draft` validation, not `Birth`.
    pub fn empty_draft(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }
}

/// One pipeline step (“penguin”).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PenguinSpec {
    pub name: String,
    pub kind: String,
    pub display_name: String,
    pub description: String,
    /// Body written to `prompts/<name>.md` (may be empty for deterministic kinds).
    pub prompt_body: String,
    /// Optional extra markdown in `agents/<name>.md` after frontmatter.
    #[serde(default)]
    pub agent_body: Option<String>,
    /// Path relative to workdir (e.g. `debug/stage-collect.md`).
    pub output: String,
    #[serde(default)]
    pub context_paths: Vec<String>,
    /// Tool names allowed for this step (`tool_ids` in agent frontmatter).
    #[serde(default)]
    pub tool_ids: Vec<String>,
    /// Pinned model (`model_id` in agent frontmatter; omitted = deployment default).
    #[serde(default)]
    pub model_id: Option<String>,
    /// Pre-run operator form fields (`[[inputs]]` in agent frontmatter).
    #[serde(default)]
    pub inputs: Vec<OperatorInputField>,
    /// UI avatar id (e.g. `ingest`, `mock_builder`); maps to `ui/src/assets/pinguins/<id>.png`.
    #[serde(default)]
    pub avatar: Option<String>,
    /// Capability override (`capability` in agent frontmatter; default `<prefix>.<kind>`).
    #[serde(default)]
    pub capability: Option<String>,
    /// Worker id override (`default_agent_id` in agent frontmatter; default `<prefix>-<kind>`).
    #[serde(default)]
    pub default_agent_id: Option<String>,
    /// Shell command for `kind = "verify"`.
    #[serde(default)]
    pub verify_command: Option<String>,
    /// Working directory relative to operator `project_path` (verify).
    #[serde(default)]
    pub verify_cwd: Option<String>,
    /// `dry-run` (default) or `execute` for `kind = "apply"`.
    #[serde(default)]
    pub apply_mode: Option<String>,
    /// `in_process` (default) or `process` step isolation.
    #[serde(default)]
    pub isolation: Option<String>,
    #[serde(default)]
    pub normalize_doc_title: Option<String>,
    #[serde(default)]
    pub normalize_sections: Vec<String>,
    #[serde(default)]
    pub normalize_fallback: Option<String>,
    #[serde(default)]
    pub normalize_fallback_sections: Vec<String>,
}

/// Options for writing a born horde to disk.
#[derive(Debug, Clone)]
pub struct HordeBirthSpec {
    pub draft: RookeryDraft,
    /// When false, refuse to write if `<output_root>/<id>/` already exists.
    pub overwrite: bool,
}

impl HordeBirthSpec {
    pub fn new(draft: RookeryDraft) -> Self {
        Self {
            draft,
            overwrite: false,
        }
    }

    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}
