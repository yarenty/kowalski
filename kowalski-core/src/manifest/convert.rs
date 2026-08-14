//! Converters between the markdown horde directory (authoring format) and the JSON
//! [`WorkflowManifest`] (interchange format).
//!
//! `horde-dir → manifest` parses the tree into a [`RookeryDraft`] pivot (prompt files
//! inlined), then projects it onto the manifest model with writer defaults resolved so
//! the manifest is self-contained. `manifest → horde-dir` reverses onto a draft and
//! regenerates the tree through the existing rookery writer path.

use crate::error::KowalskiError;
use crate::manifest::model::{
    BUILTIN_TOOL_PROVIDER, MANIFEST_SCHEMA_VERSION, ManifestExtension, ManifestStep, StepAgent,
    StepExtension, ToolBinding, WorkflowManifest,
};
use crate::manifest::validate::validate_manifest;
use crate::markdown_pipeline::{load_stage_agents, parse_app_manifest, resolve_manifest_path};
use crate::operator_input::OperatorInputField;
use crate::rookery::{
    HordeBirthSpec, PenguinSpec, RookeryDraft, default_agent_body, effective_context_paths,
    resolve_horde_fields, write_horde_tree,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

/// Fallback workflow version when the horde dir declares none.
pub const DEFAULT_MANIFEST_VERSION: &str = "0.1.0";

/// Export a horde directory as a portable manifest (prompt files inlined).
pub fn horde_dir_to_manifest(root: &Path) -> Result<WorkflowManifest, KowalskiError> {
    Ok(manifest_from_draft(&draft_from_horde_dir(root)?))
}

/// Regenerate a horde directory from a manifest via the rookery writer.
/// Validates at publish strictness first; returns the written horde root.
pub fn write_manifest_tree(
    output_root: &Path,
    manifest: &WorkflowManifest,
    overwrite: bool,
) -> Result<PathBuf, KowalskiError> {
    validate_manifest(manifest)?;
    let draft = manifest_to_draft(manifest)?;
    write_horde_tree(
        output_root,
        &HordeBirthSpec::new(draft).with_overwrite(overwrite),
    )
}

/// Parse a horde directory (`horde.md` + `agents/*.md` + `prompts/*.md`) into a draft.
pub fn draft_from_horde_dir(root: &Path) -> Result<RookeryDraft, KowalskiError> {
    let meta = parse_app_manifest(&resolve_manifest_path(root))?;
    let agents = load_stage_agents(&root.join("agents"))?;
    let prefix = meta
        .capability_prefix
        .clone()
        .unwrap_or_else(|| meta.id.clone());

    let mut penguins = Vec::with_capacity(meta.pipeline.len());
    for name in &meta.pipeline {
        let stage = agents.get(name).ok_or_else(|| {
            KowalskiError::Validation(format!(
                "pipeline references missing agent definition `{name}` (expected agents/{name}.md)"
            ))
        })?;
        let prompt_body = match stage.prompt_file.as_deref() {
            Some(rel) => fs::read_to_string(root.join(rel))
                .map_err(|e| KowalskiError::Validation(format!("read prompt {rel}: {e}")))?,
            None => String::new(),
        };
        let agent_body = read_agent_body(&root.join("agents").join(format!("{name}.md")))?;
        let derived_capability = format!("{}.{}", prefix, stage.kind);
        let derived_agent_id = format!("{}-{}", prefix.replace('.', "-"), stage.kind);
        let mut penguin = PenguinSpec {
            name: stage.name.clone(),
            kind: stage.kind.clone(),
            display_name: stage
                .display_name
                .clone()
                .unwrap_or_else(|| stage.name.clone()),
            description: stage.description.clone().unwrap_or_default(),
            prompt_body,
            agent_body,
            output: stage.output.clone().unwrap_or_default(),
            context_paths: stage.context_paths.clone(),
            tool_ids: stage.tool_ids.clone(),
            model_id: stage.model_id.clone(),
            inputs: stage.inputs.clone(),
            avatar: stage.avatar.clone(),
            capability: stage
                .capability
                .clone()
                .filter(|c| *c != derived_capability),
            default_agent_id: stage
                .default_agent_id
                .clone()
                .filter(|a| *a != derived_agent_id),
            verify_command: stage.verify_command.clone(),
            verify_cwd: stage.verify_cwd.clone(),
            apply_mode: stage.apply_mode.clone(),
            isolation: stage.isolation.clone(),
            normalize_doc_title: stage.normalize_doc_title.clone(),
            normalize_sections: stage.normalize_sections.clone(),
            normalize_fallback: stage.normalize_fallback.clone(),
            normalize_fallback_sections: stage.normalize_fallback_sections.clone(),
        };
        // A body identical to the writer default is derived content, not authored.
        if penguin.agent_body.as_deref() == Some(default_agent_body(&penguin).as_str()) {
            penguin.agent_body = None;
        }
        penguins.push(penguin);
    }

    Ok(RookeryDraft {
        id: meta.id.clone(),
        display_name: meta.display_name.clone().unwrap_or_else(|| meta.id.clone()),
        description: meta.description.clone().unwrap_or_default(),
        version: meta.version.clone(),
        capability_prefix: meta.capability_prefix.clone(),
        pipeline: meta.pipeline.clone(),
        edges: meta.edges.clone(),
        triggers: meta.triggers.clone(),
        penguins,
        default_question: meta.default_question.clone(),
        default_topic: meta.default_topic.clone(),
        workdir: meta.workdir.clone(),
        delivery_title: meta.delivery_title.clone(),
        delivery_note: meta.delivery_note.clone(),
        delivery_root_rel: meta.delivery_root_rel.clone(),
        delivery_summary_note: meta.delivery_summary_note.clone(),
        prompt_tip: meta.prompt_tip.clone(),
    })
}

/// Project a draft onto the manifest model, resolving writer defaults so the
/// manifest is self-contained.
pub fn manifest_from_draft(draft: &RookeryDraft) -> WorkflowManifest {
    let resolved = resolve_horde_fields(draft);
    let steps = draft
        .pipeline
        .iter()
        .filter_map(|name| draft.penguins.iter().find(|p| &p.name == name))
        .map(|p| step_from_penguin(draft, p))
        .collect();

    let extension = ManifestExtension {
        capability_prefix: Some(resolved.capability_prefix),
        default_question: Some(resolved.default_question),
        default_topic: Some(resolved.default_topic),
        workdir: Some(resolved.workdir),
        delivery_title: Some(resolved.delivery_title),
        delivery_note: Some(resolved.delivery_note),
        delivery_root_rel: Some(resolved.delivery_root_rel),
        delivery_summary_note: Some(resolved.delivery_summary_note),
        prompt_tip: (!resolved.prompt_tip.is_empty()).then_some(resolved.prompt_tip),
        triggers: draft.triggers.clone(),
    };

    WorkflowManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.into(),
        identifier: draft.id.clone(),
        name: draft.display_name.clone(),
        version: draft
            .version
            .clone()
            .unwrap_or_else(|| DEFAULT_MANIFEST_VERSION.into()),
        description: draft.description.clone(),
        steps,
        pipeline: draft.pipeline.clone(),
        edges: draft.edges.clone(),
        kowalski: Some(extension),
    }
}

/// Reverse a manifest onto a draft ready for the rookery writer.
pub fn manifest_to_draft(manifest: &WorkflowManifest) -> Result<RookeryDraft, KowalskiError> {
    let ext = manifest.kowalski.clone().unwrap_or_default();
    let mut penguins = Vec::with_capacity(manifest.steps.len());
    for step in &manifest.steps {
        penguins.push(penguin_from_step(step)?);
    }
    Ok(RookeryDraft {
        id: manifest.identifier.clone(),
        display_name: manifest.name.clone(),
        description: manifest.description.clone(),
        version: Some(manifest.version.clone()),
        capability_prefix: ext.capability_prefix,
        pipeline: manifest.pipeline.clone(),
        edges: manifest.edges.clone(),
        triggers: ext.triggers,
        penguins,
        default_question: ext.default_question,
        default_topic: ext.default_topic,
        workdir: ext.workdir,
        delivery_title: ext.delivery_title,
        delivery_note: ext.delivery_note,
        delivery_root_rel: ext.delivery_root_rel,
        delivery_summary_note: ext.delivery_summary_note,
        prompt_tip: ext.prompt_tip,
    })
}

fn step_from_penguin(draft: &RookeryDraft, penguin: &PenguinSpec) -> ManifestStep {
    let agent =
        (!penguin.prompt_body.trim().is_empty() || penguin.model_id.is_some()).then(|| StepAgent {
            system_prompt: penguin.prompt_body.clone(),
            model: penguin.model_id.clone(),
            parameters: None,
        });
    let tool_bindings = if penguin.tool_ids.is_empty() {
        Vec::new()
    } else {
        vec![ToolBinding {
            provider: BUILTIN_TOOL_PROVIDER.into(),
            tools: Some(penguin.tool_ids.clone()),
            overrides: None,
        }]
    };
    let ui = (!penguin.inputs.is_empty()).then(|| json!({ "inputs": penguin.inputs }));
    let extension = StepExtension {
        output: (!penguin.output.is_empty()).then(|| penguin.output.clone()),
        context_paths: effective_context_paths(draft, penguin),
        capability: penguin.capability.clone(),
        default_agent_id: penguin.default_agent_id.clone(),
        avatar: penguin.avatar.clone(),
        isolation: penguin.isolation.clone(),
        verify_command: penguin.verify_command.clone(),
        verify_cwd: penguin.verify_cwd.clone(),
        apply_mode: penguin.apply_mode.clone(),
        normalize_doc_title: penguin.normalize_doc_title.clone(),
        normalize_sections: penguin.normalize_sections.clone(),
        normalize_fallback: penguin.normalize_fallback.clone(),
        normalize_fallback_sections: penguin.normalize_fallback_sections.clone(),
        agent_body: penguin.agent_body.clone(),
    };
    ManifestStep {
        id: penguin.name.clone(),
        kind: penguin.kind.clone(),
        name: penguin.display_name.clone(),
        description: penguin.description.clone(),
        agent,
        tool_bindings,
        input: None,
        ui,
        kowalski: (!extension.is_empty()).then_some(extension),
    }
}

fn penguin_from_step(step: &ManifestStep) -> Result<PenguinSpec, KowalskiError> {
    let ext = step.kowalski.clone().unwrap_or_default();
    let inputs: Vec<OperatorInputField> = match step.ui.as_ref().and_then(|ui| ui.get("inputs")) {
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
            KowalskiError::Validation(format!(
                "step `{}`: ui.inputs does not parse as operator-form fields: {e}",
                step.id
            ))
        })?,
        None => Vec::new(),
    };
    let tool_ids = step
        .tool_bindings
        .iter()
        .flat_map(|b| b.tools.clone().unwrap_or_default())
        .collect();
    Ok(PenguinSpec {
        name: step.id.clone(),
        kind: step.kind.clone(),
        display_name: step.name.clone(),
        description: step.description.clone(),
        prompt_body: step
            .agent
            .as_ref()
            .map(|a| a.system_prompt.clone())
            .unwrap_or_default(),
        agent_body: ext.agent_body,
        output: ext.output.unwrap_or_default(),
        context_paths: ext.context_paths,
        tool_ids,
        model_id: step.agent.as_ref().and_then(|a| a.model.clone()),
        inputs,
        avatar: ext.avatar,
        capability: ext.capability,
        default_agent_id: ext.default_agent_id,
        verify_command: ext.verify_command,
        verify_cwd: ext.verify_cwd,
        apply_mode: ext.apply_mode,
        isolation: ext.isolation,
        normalize_doc_title: ext.normalize_doc_title,
        normalize_sections: ext.normalize_sections,
        normalize_fallback: ext.normalize_fallback,
        normalize_fallback_sections: ext.normalize_fallback_sections,
    })
}

/// Markdown body of an agent file after the closing frontmatter fence.
fn read_agent_body(path: &Path) -> Result<Option<String>, KowalskiError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| KowalskiError::Validation(format!("read {}: {e}", path.display())))?;
    let mut fences = 0usize;
    let mut body_start = None;
    let mut offset = 0usize;
    for line in raw.split_inclusive('\n') {
        offset += line.len();
        if line.trim() == "---" {
            fences += 1;
            if fences == 2 {
                body_start = Some(offset);
                break;
            }
        }
    }
    let body = body_start
        .map(|start| raw[start..].trim_start_matches('\n').to_string())
        .filter(|b| !b.trim().is_empty());
    Ok(body)
}
