//! Canonical JSON workflow-manifest model (interchange/runtime format).
//!
//! The markdown horde directory (`horde.md` + `agents/*.md` + `prompts/*.md`) stays the
//! authoring format; a [`WorkflowManifest`] is its portable JSON projection: exported,
//! imported, validated, and consumed by other systems. Every struct is
//! `deny_unknown_fields` so a manifest either matches this contract exactly or fails
//! to parse. The published JSON Schema asset mirrors this model
//! (`resources/schemas/workflow-manifest.schema.json`).

use crate::horde_graph::HordeEdge;
use crate::horde_trigger::HordeTrigger;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

/// Manifest contract version (`MAJOR.MINOR`) written by the exporter.
pub const MANIFEST_SCHEMA_VERSION: &str = "1.0";

/// Supported manifest MAJOR version; newer MAJOR is rejected by validation.
pub const MANIFEST_SCHEMA_MAJOR: u32 = 1;

/// Tool-binding provider name for kowalski's flat built-in `tool_ids`.
pub const BUILTIN_TOOL_PROVIDER: &str = "builtin";

/// The canonical workflow-manifest JSON Schema asset, embedded verbatim.
pub const WORKFLOW_MANIFEST_SCHEMA_JSON: &str =
    include_str!("../../resources/schemas/workflow-manifest.schema.json");

/// The canonical workflow-manifest schema, parsed once.
pub fn workflow_manifest_schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(WORKFLOW_MANIFEST_SCHEMA_JSON)
            .expect("embedded workflow-manifest schema must be valid JSON")
    })
}

/// Portable workflow manifest: one workflow (horde), its steps, and its scheduling graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowManifest {
    /// Manifest contract version, `MAJOR.MINOR` (see [`MANIFEST_SCHEMA_VERSION`]).
    pub schema_version: String,
    /// Kebab-case workflow id (`^[a-z][a-z0-9-]*[a-z0-9]$`); the horde id on import.
    pub identifier: String,
    /// Human-readable workflow name (horde `display_name`).
    pub name: String,
    /// Workflow semver (`x.y.z`); defaults to `0.1.0` on export when the dir declares none.
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub steps: Vec<ManifestStep>,
    /// Step ids in topological order; must cover `steps[]` exactly.
    pub pipeline: Vec<String>,
    /// Scheduling DAG; absent or empty → implicit chain along `pipeline` order.
    /// One-to-one with the horde `[[edges]]` declaration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<HordeEdge>,
    /// Kowalski extension block: horde fields with no generic manifest home
    /// (triggers, delivery presentation, run defaults).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kowalski: Option<ManifestExtension>,
}

/// One workflow step (horde pipeline stage).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ManifestStep {
    /// Step id (kebab-case), unique within the manifest; referenced by `pipeline`/`edges`.
    pub id: String,
    /// Step kind. Known kinds are declared in the schema enum; unknown kinds are an
    /// import-portability concern, not a parse failure.
    pub kind: String,
    /// Human-readable step name (agent `display_name`).
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// LLM agent config; prompt file contents are inlined into `system_prompt` on export.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<StepAgent>,
    /// Tool access declarations (never credentials).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_bindings: Vec<ToolBinding>,
    /// Config for `kind = "input"` steps (interchange vocabulary; kowalski ingest
    /// steps use the `ui` operator form instead).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<StepInput>,
    /// Free-form UI hints. Kowalski convention: `{"inputs": [...]}` carries the
    /// `[[inputs]]` operator-form fields verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<Value>,
    /// Kowalski extension block: step fields with no generic manifest home
    /// (artifact output path, context wiring, verify/apply config, isolation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kowalski: Option<StepExtension>,
}

/// LLM agent configuration for a step.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StepAgent {
    /// Full prompt text (prompt file inlined on export).
    pub system_prompt: String,
    /// Pinned model id; omitted = deployment default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Free-form generation parameters (temperature, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

/// Tool access declaration: which provider, optionally which of its tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ToolBinding {
    /// Tool source (kowalski exports its flat `tool_ids` under [`BUILTIN_TOOL_PROVIDER`]).
    pub provider: String,
    /// Allowed tool names; omitted = provider default set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Free-form provider-specific settings (never credentials).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Value>,
}

/// Config for `kind = "input"` steps.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StepInput {
    /// Prompt shown when asking for the input.
    pub prompt: String,
    /// Optional JSON Schema constraining the expected value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

/// Manifest-level kowalski extension: horde.md fields beyond the generic contract.
/// Export resolves writer defaults first, so a manifest is self-contained.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ManifestExtension {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_root_rel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_summary_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tip: Option<String>,
    /// Event-driven run declarations, every field carried (incl. `overlap`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<HordeTrigger>,
}

/// Step-level kowalski extension: agent frontmatter fields beyond the generic contract.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StepExtension {
    /// Artifact path relative to workdir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Context wiring (`@artifact@`, `@step:<id>@`, or workdir-relative paths).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// `in_process` (default) or `process` step isolation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,
    /// Shell command for `kind = "verify"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_cwd: Option<String>,
    /// `dry-run` (default) or `execute` for `kind = "apply"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalize_doc_title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normalize_sections: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalize_fallback: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normalize_fallback_sections: Vec<String>,
    /// Extra markdown body of `agents/<id>.md` after the frontmatter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_body: Option<String>,
}

impl StepExtension {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

impl ManifestExtension {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}
