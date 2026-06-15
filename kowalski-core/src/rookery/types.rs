//! Rookery draft types (linear pipeline in 1.3.0; optional `edges[]` in 1.4.0+).

use crate::horde_graph::HordeEdge;
use crate::operator_input::OperatorInputField;
use serde::{Deserialize, Serialize};

/// In-memory draft between interview and **Give birth** (linear `pipeline` order only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RookeryDraft {
    pub id: String,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub capability_prefix: Option<String>,
    /// Ordered step names; must match `penguins` keys exactly.
    pub pipeline: Vec<String>,
    /// Optional DAG edges; absent or empty → implicit chain along `pipeline`.
    #[serde(default)]
    pub edges: Vec<HordeEdge>,
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

/// One pipeline step (“penguin”).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PenguinSpec {
    pub name: String,
    pub kind: String,
    pub display_name: String,
    pub description: String,
    /// Body written to `prompts/<name>.md`.
    pub prompt_body: String,
    /// Optional extra markdown in `agents/<name>.md` after frontmatter.
    #[serde(default)]
    pub agent_body: Option<String>,
    /// Path relative to workdir (e.g. `debug/stage-collect.md`).
    pub output: String,
    #[serde(default)]
    pub context_paths: Vec<String>,
    /// Reserved for UI / future runtime wiring (not written to agent frontmatter in 1.3.0).
    #[serde(default)]
    pub tool_ids: Vec<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    /// Pre-run operator form fields (`[[inputs]]` in agent frontmatter).
    #[serde(default)]
    pub inputs: Vec<OperatorInputField>,
    /// UI avatar id (e.g. `ingest`, `mock_builder`); maps to `ui/src/assets/pinguins/<id>.png`.
    #[serde(default)]
    pub avatar: Option<String>,
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
