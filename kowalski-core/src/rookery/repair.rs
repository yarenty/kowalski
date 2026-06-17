//! Repair invalid `output` paths in an on-disk horde tree (e.g. LLM wrote `String`).

use crate::error::KowalskiError;
use crate::markdown_pipeline::{parse_app_manifest, parse_stage_agent, resolve_manifest_path};
use crate::rookery::normalize::{default_output_for_penguin, output_looks_invalid};
use crate::rookery::types::{PenguinSpec, RookeryDraft};
use std::fs;
use std::path::Path;

/// Fix placeholder `output` values under `agents/*.md` and invalid `delivery_root_rel` in `horde.md`.
pub fn repair_horde_tree_outputs(root: &Path) -> Result<u32, KowalskiError> {
    let manifest_path = resolve_manifest_path(root);
    let raw_horde = fs::read_to_string(&manifest_path)?;
    let meta = parse_app_manifest(&manifest_path)?;
    let mut fixed = 0u32;

    let delivery_key = "delivery_root_rel";
    if let Some(line) = raw_horde.lines().find(|l| l.trim_start().starts_with(delivery_key)) {
        let val = line
            .split('=')
            .nth(1)
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_default();
        if output_looks_invalid(&val) {
            let updated = raw_horde.replace(
                line,
                &format!("{delivery_key} = \"HANDOFF.md\""),
            );
            fs::write(&manifest_path, updated)?;
            fixed += 1;
        }
    }

    let draft = RookeryDraft {
        id: meta.id.clone(),
        display_name: meta.display_name.clone().unwrap_or_else(|| meta.id.clone()),
        description: String::new(),
        capability_prefix: None,
        pipeline: meta.pipeline.clone(),
        edges: meta.edges.clone(),
        penguins: vec![],
        default_question: meta.default_question.clone(),
        default_topic: None,
        workdir: Some("output".into()),
        delivery_title: None,
        delivery_note: None,
        delivery_root_rel: Some("HANDOFF.md".into()),
        delivery_summary_note: None,
        prompt_tip: None,
    };

    let pipeline = meta.pipeline.clone();
    let n = pipeline.len();
    for (i, name) in pipeline.iter().enumerate() {
        let path = root.join("agents").join(format!("{name}.md"));
        if !path.is_file() {
            continue;
        }
        let stage = parse_stage_agent(&path)?;
        let out = stage.output.as_deref().unwrap_or("");
        if !output_looks_invalid(out) {
            continue;
        }
        let penguin = PenguinSpec {
            name: stage.name.clone(),
            kind: stage.kind.clone(),
            display_name: stage.name.clone(),
            description: String::new(),
            prompt_body: String::new(),
            agent_body: None,
            output: out.to_string(),
            context_paths: stage.context_paths.clone(),
            tool_ids: vec![],
            model_id: None,
            inputs: stage.inputs.clone(),
            avatar: None,
        };
        let is_first = i == 0;
        let is_last = i == n - 1;
        let new_out = default_output_for_penguin(
            draft.delivery_root_rel.as_deref(),
            &penguin,
            is_first,
            is_last,
        );
        let raw = fs::read_to_string(&path)?;
        let updated = replace_frontmatter_output_line(&raw, &new_out)?;
        fs::write(&path, updated)?;
        fixed += 1;
    }
    Ok(fixed)
}

fn replace_frontmatter_output_line(raw: &str, new_output: &str) -> Result<String, KowalskiError> {
    let mut out = String::new();
    let mut in_fm = false;
    let mut replaced = false;
    for line in raw.lines() {
        if line.trim() == "---" {
            in_fm = !in_fm || out.is_empty();
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_fm && line.trim_start().starts_with("output") {
            out.push_str(&format!("output = \"{}\"\n", escape_toml(new_output)));
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        return Err(KowalskiError::Validation(
            "no output = line in agent frontmatter".into(),
        ));
    }
    Ok(out)
}

fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\"', "\\\"")
}
