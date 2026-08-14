//! **Workflow manifest** — canonical JSON interchange form of a horde.
//!
//! The markdown directory (`horde.md` + `agents/*.md` + `prompts/*.md`) stays the
//! authoring format; the [`WorkflowManifest`] is the export/import/runtime projection:
//! strict serde model ([`model`]), semantic validation ([`validate`]), and bidirectional
//! converters ([`convert`]). The published contract is the JSON Schema asset at
//! `resources/schemas/workflow-manifest.schema.json` (embedded as
//! [`WORKFLOW_MANIFEST_SCHEMA_JSON`]). Manifests never contain credentials, run
//! history, or run document contents.

mod convert;
mod model;
mod validate;

pub use convert::{
    DEFAULT_MANIFEST_VERSION, draft_from_horde_dir, horde_dir_to_manifest, manifest_from_draft,
    manifest_to_draft, write_manifest_tree,
};
pub use model::{
    BUILTIN_TOOL_PROVIDER, MANIFEST_SCHEMA_MAJOR, MANIFEST_SCHEMA_VERSION, ManifestExtension,
    ManifestStep, StepAgent, StepExtension, StepInput, ToolBinding, WORKFLOW_MANIFEST_SCHEMA_JSON,
    WorkflowManifest, workflow_manifest_schema,
};
pub use validate::{
    DETERMINISTIC_STEP_KINDS, INTERCHANGE_STEP_KINDS, ManifestStrictness, validate_manifest,
    validate_manifest_with,
};

#[cfg(test)]
mod tests;
