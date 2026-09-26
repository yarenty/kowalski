//! Semantic validation for [`WorkflowManifest`] beyond what serde/JSON Schema enforce:
//! id uniqueness, pipeline coverage, edge resolution, kind-config exclusivity, and
//! draft-vs-publish strictness.

use crate::error::KowalskiError;
use crate::horde_graph::resolve_execution_graph;
use crate::horde_step::LLM_STEP_KINDS;
use crate::horde_trigger::validate_triggers;
use crate::manifest::model::{MANIFEST_SCHEMA_MAJOR, ManifestStep, WorkflowManifest};
use crate::operator_input::OperatorInputField;
use crate::rookery::{validate_horde_id, validate_step_name};
use std::collections::BTreeSet;

/// Deterministic (non-LLM) kowalski step kinds executed by the in-process registry.
pub const DETERMINISTIC_STEP_KINDS: &[&str] = &["verify", "apply", "ingest", "table_profile", "sql_batch", "xlsx_report"];

/// Interchange kinds declared for cross-system manifests; not executed by kowalski.
pub const INTERCHANGE_STEP_KINDS: &[&str] = &["llm", "rag", "input"];

/// How complete a manifest must be to pass [`validate_manifest_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestStrictness {
    /// In-progress manifest: empty name/description/prompts allowed while structural
    /// integrity (ids, pipeline coverage, edges, kind-config exclusivity) is enforced.
    Draft,
    /// Everything required for export/import of a runnable workflow.
    Publish,
}

/// Validate at [`ManifestStrictness::Publish`].
pub fn validate_manifest(manifest: &WorkflowManifest) -> Result<Vec<String>, KowalskiError> {
    validate_manifest_with(manifest, ManifestStrictness::Publish)
}

/// Validate a manifest; returns non-fatal warnings (e.g. unknown step kinds, which are
/// an import-portability concern rather than an error).
pub fn validate_manifest_with(
    manifest: &WorkflowManifest,
    strictness: ManifestStrictness,
) -> Result<Vec<String>, KowalskiError> {
    let lenient = strictness == ManifestStrictness::Draft;
    let mut errs = Vec::new();
    let mut warnings = Vec::new();

    match parse_schema_version(&manifest.schema_version) {
        Ok((major, _minor)) => {
            if major != MANIFEST_SCHEMA_MAJOR {
                errs.push(format!(
                    "unsupported schema_version `{}` (supported MAJOR: {})",
                    manifest.schema_version, MANIFEST_SCHEMA_MAJOR
                ));
            }
        }
        Err(e) => errs.push(e),
    }

    if let Err(e) = validate_horde_id(&manifest.identifier) {
        errs.push(format!("identifier: {e}"));
    }
    if !lenient && manifest.name.trim().is_empty() {
        errs.push("name must not be empty".into());
    }
    if !lenient && !is_semver(&manifest.version) {
        errs.push(format!(
            "version `{}` is not a semver (`x.y.z`)",
            manifest.version
        ));
    }
    if !lenient && manifest.steps.is_empty() {
        errs.push("steps must contain at least one step".into());
    }

    let mut step_ids = BTreeSet::new();
    for step in &manifest.steps {
        if !step_ids.insert(step.id.clone()) {
            errs.push(format!("duplicate step id `{}`", step.id));
        }
        if let Err(e) = validate_step_name(&step.id) {
            errs.push(format!("step id: {e}"));
        }
        validate_step(step, lenient, &mut errs, &mut warnings);
    }

    let mut seen_pipe = BTreeSet::new();
    for id in &manifest.pipeline {
        if !seen_pipe.insert(id.clone()) {
            errs.push(format!("duplicate pipeline entry `{id}`"));
        }
        if !step_ids.contains(id) {
            errs.push(format!("pipeline references missing step `{id}`"));
        }
    }
    for id in &step_ids {
        if !seen_pipe.contains(id) {
            errs.push(format!("step `{id}` is not listed in pipeline"));
        }
    }

    if errs.is_empty() && !(manifest.pipeline.is_empty() && manifest.edges.is_empty()) {
        let edge_slice = if manifest.edges.is_empty() {
            None
        } else {
            Some(manifest.edges.as_slice())
        };
        if let Err(e) = resolve_execution_graph(&manifest.pipeline, edge_slice) {
            errs.push(e.to_string());
        }
    }

    if let Some(ext) = &manifest.kowalski {
        match validate_triggers(&ext.triggers, None) {
            Ok(w) => warnings.extend(w),
            Err(e) => errs.push(e.to_string()),
        }
    }

    if errs.is_empty() {
        Ok(warnings)
    } else {
        Err(KowalskiError::Validation(errs.join("; ")))
    }
}

/// Whether this deployment recognizes a step kind (executable or interchange).
pub fn is_known_step_kind(kind: &str) -> bool {
    LLM_STEP_KINDS.contains(&kind)
        || DETERMINISTIC_STEP_KINDS.contains(&kind)
        || INTERCHANGE_STEP_KINDS.contains(&kind)
}

fn validate_step(
    step: &ManifestStep,
    lenient: bool,
    errs: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if !is_known_step_kind(&step.kind) {
        warnings.push(format!(
            "step `{}`: unknown kind `{}` (portability: not executable by this deployment)",
            step.id, step.kind
        ));
    }

    if !lenient && step.name.trim().is_empty() {
        errs.push(format!("step `{}`: name must not be empty", step.id));
    }

    let is_llm = LLM_STEP_KINDS.contains(&step.kind.as_str()) || step.kind == "llm";
    if !lenient
        && is_llm
        && step
            .agent
            .as_ref()
            .is_none_or(|a| a.system_prompt.trim().is_empty())
    {
        errs.push(format!(
            "step `{}`: kind `{}` requires agent.system_prompt",
            step.id, step.kind
        ));
    }

    // Kind-config exclusivity: config blocks only make sense on their kind.
    if step.input.is_some() && step.kind != "input" {
        errs.push(format!(
            "step `{}`: input block is only valid on kind `input` (kind is `{}`)",
            step.id, step.kind
        ));
    }
    if let Some(ext) = &step.kowalski {
        if (ext.verify_command.is_some() || ext.verify_cwd.is_some()) && step.kind != "verify" {
            errs.push(format!(
                "step `{}`: verify_command/verify_cwd are only valid on kind `verify` (kind is `{}`)",
                step.id, step.kind
            ));
        }
        if ext.apply_mode.is_some() && step.kind != "apply" {
            errs.push(format!(
                "step `{}`: apply_mode is only valid on kind `apply` (kind is `{}`)",
                step.id, step.kind
            ));
        }
    }

    for binding in &step.tool_bindings {
        if binding.provider.trim().is_empty() {
            errs.push(format!(
                "step `{}`: tool_bindings provider must not be empty",
                step.id
            ));
        }
    }

    // The kowalski ui convention (`{"inputs": [...]}`) must parse as operator-form fields.
    if let Some(inputs) = step.ui.as_ref().and_then(|ui| ui.get("inputs"))
        && serde_json::from_value::<Vec<OperatorInputField>>(inputs.clone()).is_err()
    {
        errs.push(format!(
            "step `{}`: ui.inputs does not parse as operator-form fields",
            step.id
        ));
    }
}

pub(crate) fn parse_schema_version(v: &str) -> Result<(u32, u32), String> {
    let bad = || format!("schema_version `{v}` must be `MAJOR.MINOR`");
    let (major, minor) = v.split_once('.').ok_or_else(bad)?;
    Ok((
        major.parse::<u32>().map_err(|_| bad())?,
        minor.parse::<u32>().map_err(|_| bad())?,
    ))
}

fn is_semver(v: &str) -> bool {
    let core = v.split_once('-').map(|(c, _pre)| c).unwrap_or(v);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}
