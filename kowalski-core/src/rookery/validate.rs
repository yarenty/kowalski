//! Validation for Rookery drafts and on-disk horde trees.

use crate::error::KowalskiError;
use crate::markdown_pipeline::{parse_app_manifest, parse_stage_agent, resolve_manifest_path};
use crate::rookery::types::RookeryDraft;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Horde / step ids: lowercase alphanumeric + hyphen; must not be empty.
pub fn validate_horde_id(id: &str) -> Result<(), KowalskiError> {
    if id.is_empty() {
        return Err(KowalskiError::Validation("horde id must not be empty".into()));
    }
    if id == "." || id == ".." {
        return Err(KowalskiError::Validation(format!(
            "invalid horde id `{id}`"
        )));
    }
    if id.contains('/') || id.contains('\\') {
        return Err(KowalskiError::Validation(format!(
            "horde id must not contain path separators: `{id}`"
        )));
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok || !id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
        return Err(KowalskiError::Validation(format!(
            "horde id must match [a-z0-9][a-z0-9-]*: `{id}`"
        )));
    }
    Ok(())
}

/// Step names use the same rules as horde ids.
pub fn validate_step_name(name: &str) -> Result<(), KowalskiError> {
    validate_horde_id(name)
}

/// Reject paths that escape a workdir via `..` or absolute roots.
pub fn validate_workdir_relative_path(rel: &str) -> Result<(), KowalskiError> {
    if rel.is_empty() {
        return Err(KowalskiError::Validation(
            "workdir-relative path must not be empty".into(),
        ));
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        return Err(KowalskiError::Validation(format!(
            "path must be relative to workdir, not absolute: `{rel}`"
        )));
    }
    let p = Path::new(rel);
    for comp in p.components() {
        if comp == std::path::Component::ParentDir {
            return Err(KowalskiError::Validation(format!(
                "path must not contain `..`: `{rel}`"
            )));
        }
    }
    Ok(())
}

/// Validate a draft before **Give birth** (linear pipeline only).
pub fn validate_draft(draft: &RookeryDraft) -> Result<(), KowalskiError> {
    let mut errs = Vec::new();

    if let Err(e) = validate_horde_id(&draft.id) {
        errs.push(e.to_string());
    }
    if draft.display_name.trim().is_empty() {
        errs.push("display_name must not be empty".into());
    }
    if draft.pipeline.is_empty() {
        errs.push("pipeline must contain at least one step".into());
    }

    let mut seen_pipe = BTreeSet::new();
    for step in &draft.pipeline {
        if !seen_pipe.insert(step.clone()) {
            errs.push(format!("duplicate pipeline step `{step}`"));
        }
        if let Err(e) = validate_step_name(step) {
            errs.push(e.to_string());
        }
    }

    let penguin_map: BTreeMap<_, _> = draft
        .penguins
        .iter()
        .map(|p| (p.name.clone(), p))
        .collect();

    if penguin_map.len() != draft.penguins.len() {
        errs.push("duplicate penguin names in penguins[]".into());
    }

    for step in &draft.pipeline {
        if !penguin_map.contains_key(step) {
            errs.push(format!(
                "pipeline references missing penguin `{step}` (expected in penguins[])"
            ));
        }
    }
    for name in penguin_map.keys() {
        if !draft.pipeline.contains(name) {
            errs.push(format!(
                "penguin `{name}` is not listed in pipeline (remove or add to pipeline)"
            ));
        }
    }

    for p in &draft.penguins {
        if let Err(e) = validate_step_name(&p.name) {
            errs.push(format!("penguin name: {e}"));
        }
        if p.prompt_body.trim().is_empty() {
            errs.push(format!("penguin `{}`: prompt_body must not be empty", p.name));
        }
        if let Err(e) = validate_workdir_relative_path(&p.output) {
            errs.push(format!("penguin `{}`: {}", p.name, e));
        }
        for ctx in &p.context_paths {
            if ctx.trim() == "@artifact@" || ctx.trim().starts_with("@step:") {
                continue;
            }
            if let Err(e) = validate_workdir_relative_path(ctx) {
                errs.push(format!("penguin `{}` context_paths: {}", p.name, e));
            }
        }
    }

    if let Some(w) = &draft.workdir {
        if let Err(e) = validate_workdir_relative_path(w) {
            errs.push(format!("workdir: {e}"));
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(KowalskiError::Validation(errs.join("; ")))
    }
}

/// Validate an on-disk horde tree (`horde.md` + `agents/*.md`), same rules as `agent-app validate`.
pub fn validate_horde_tree(root: &Path) -> Result<(), KowalskiError> {
    let mut errs = Vec::new();
    let mpath = resolve_manifest_path(root);
    if !mpath.is_file() {
        return Err(KowalskiError::Validation(format!(
            "missing manifest (tried app.md and horde.md under {})",
            root.display()
        )));
    }
    let meta = parse_app_manifest(&mpath).map_err(|e| e.to_string())?;
    let agents_dir = root.join("agents");
    if !agents_dir.is_dir() {
        return Err(KowalskiError::Validation(format!(
            "agents/ missing under {}",
            root.display()
        )));
    }

    let mut defs = BTreeMap::new();
    let rd = fs::read_dir(&agents_dir).map_err(KowalskiError::Io)?;
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let stage = parse_stage_agent(&path).map_err(|e| e.to_string())?;
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        defs.insert(key, (path, stage));
    }

    let pipeline_set: BTreeSet<_> = meta.pipeline.iter().cloned().collect();
    for name in &meta.pipeline {
        if !defs.contains_key(name) {
            errs.push(format!(
                "manifest pipeline references missing agent definition `{name}` (expected agents/{name}.md)"
            ));
        }
    }
    for name in defs.keys() {
        if !pipeline_set.contains(name) {
            errs.push(format!(
                "agents/{name}.md exists but `{name}` is not listed in the manifest pipeline"
            ));
        }
    }
    for (key, (path, agent)) in &defs {
        if agent.name != *key {
            errs.push(format!(
                "agent name mismatch in {} (file `{}` vs meta `{}`)",
                path.display(),
                key,
                agent.name
            ));
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(KowalskiError::Validation(errs.join("; ")))
    }
}
