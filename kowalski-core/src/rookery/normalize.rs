//! Normalize LLM-produced ids to safe horde / step names before validation.

use crate::operator_input::default_ingest_form_fields;
use crate::rookery::avatars::assign_penguin_avatars;
use crate::rookery::types::{PenguinSpec, RookeryDraft};
use crate::rookery::validate::validate_workdir_relative_path;
use std::collections::{BTreeMap, BTreeSet};

/// Convert arbitrary text to `[a-z0-9][a-z0-9-]*` (lowercase kebab-case).
pub fn slugify_horde_id(id: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for c in id.trim().chars() {
        let ch = if c.is_ascii_alphanumeric() {
            c.to_ascii_lowercase()
        } else if matches!(c, '_' | ' ' | '.' | '/' | '\\' | '-') {
            '-'
        } else {
            continue;
        };
        if ch == '-' {
            if out.is_empty() || prev_hyphen {
                continue;
            }
            prev_hyphen = true;
            out.push('-');
        } else {
            prev_hyphen = false;
            out.push(ch);
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        return "horde".into();
    }
    if !out
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        out.insert(0, 'h');
    }
    out
}

/// Fix horde id, penguin `name`, and `pipeline` entries so they pass [`validate_draft`].
pub fn normalize_draft(draft: &mut RookeryDraft) {
    draft.id = slugify_horde_id(&draft.id);

    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut rename: BTreeMap<String, String> = BTreeMap::new();

    for p in &draft.penguins {
        let mut base = slugify_horde_id(&p.name);
        if base.is_empty() {
            base = "step".into();
        }
        let mut candidate = base.clone();
        let mut n = 2u32;
        while used.contains(&candidate) {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        used.insert(candidate.clone());
        rename.insert(p.name.clone(), candidate);
    }

    for p in &mut draft.penguins {
        if let Some(new_name) = rename.get(&p.name) {
            p.name = new_name.clone();
        }
    }

    draft.pipeline = draft
        .pipeline
        .iter()
        .map(|step| {
            rename
                .get(step)
                .cloned()
                .unwrap_or_else(|| slugify_horde_id(step))
        })
        .collect();

    normalize_delivery_root(draft);
    let pipeline = draft.pipeline.clone();
    let delivery_root = draft.delivery_root_rel.as_deref();
    for p in &mut draft.penguins {
        let is_first = pipeline.first() == Some(&p.name);
        let is_last = pipeline.last() == Some(&p.name);
        normalize_penguin_output(delivery_root, p, is_first, is_last);
        if is_first && p.inputs.is_empty() && matches!(p.kind.as_str(), "ingest" | "step") {
            p.inputs = default_ingest_form_fields();
        }
    }
    assign_penguin_avatars(draft);
}

fn normalize_delivery_root(draft: &mut RookeryDraft) {
    let fix = |s: &str| -> bool {
        let t = s.trim();
        t.is_empty()
            || t.eq_ignore_ascii_case("string")
            || (!t.contains('/') && !t.contains('.') && t.len() < 64)
    };
    if draft
        .delivery_root_rel
        .as_ref()
        .is_some_and(|r| fix(r))
    {
        draft.delivery_root_rel = Some("HANDOFF.md".into());
    }
}

/// True when `output` looks like a placeholder (e.g. LLM wrote `String`) not a workdir path.
pub fn output_looks_invalid(output: &str) -> bool {
    let t = output.trim();
    if t.is_empty() {
        return true;
    }
    if t.eq_ignore_ascii_case("string")
        || t.eq_ignore_ascii_case("path")
        || t.eq_ignore_ascii_case("output")
        || t.eq_ignore_ascii_case("artifact")
    {
        return true;
    }
    if !t.contains('/') && !t.contains('.') && t.chars().all(|c| c.is_alphabetic() || c.is_whitespace())
    {
        return true;
    }
    validate_workdir_relative_path(t).is_err()
}

pub fn default_output_for_penguin(
    delivery_root_rel: Option<&str>,
    penguin: &PenguinSpec,
    is_first: bool,
    is_last: bool,
) -> String {
    match penguin.kind.as_str() {
        "ingest" => "debug/raw/".into(),
        "deliver" => delivery_root_rel
            .map(str::to_string)
            .unwrap_or_else(|| "HANDOFF.md".into()),
        _ if is_last => delivery_root_rel
            .map(str::to_string)
            .unwrap_or_else(|| format!("debug/stage-{}.md", penguin.name)),
        _ if is_first => format!("debug/stage-{}.md", penguin.name),
        _ => format!("debug/stage-{}.md", penguin.name),
    }
}

pub fn normalize_penguin_output(
    delivery_root_rel: Option<&str>,
    penguin: &mut PenguinSpec,
    is_first: bool,
    is_last: bool,
) {
    if output_looks_invalid(&penguin.output) {
        penguin.output =
            default_output_for_penguin(delivery_root_rel, penguin, is_first, is_last);
    }
    if !is_first && penguin.context_paths.is_empty() {
        penguin.context_paths = vec!["@artifact@".into()];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rookery::fixture::minimal_linear_draft;
    use crate::rookery::types::PenguinSpec;
    use crate::rookery::validate::validate_draft;

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify_horde_id("Ingest"), "ingest");
        assert_eq!(
            slugify_horde_id("rust_project_scaffolder_1.0"),
            "rust-project-scaffolder-1-0"
        );
        assert_eq!(slugify_horde_id("  Foo Bar  "), "foo-bar");
    }

    #[test]
    fn fixes_placeholder_output_string() {
        let mut draft = minimal_linear_draft();
        draft.penguins[0].output = "String".into();
        normalize_draft(&mut draft);
        assert_eq!(draft.penguins[0].output, "debug/raw/");
        validate_draft(&draft).expect("outputs should validate");
    }

    #[test]
    fn normalize_draft_fixes_casing_and_horde_id() {
        let mut draft = RookeryDraft {
            id: "rust_project_scaffolder_1.0".into(),
            display_name: "Rust scaffolder".into(),
            description: "demo".into(),
            capability_prefix: None,
            pipeline: vec![
                "Ingest".into(),
                "Structure".into(),
                "Deliver".into(),
            ],
            penguins: vec![
                PenguinSpec {
                    name: "Ingest".into(),
                    kind: "ingest".into(),
                    display_name: "Ingest".into(),
                    description: "d".into(),
                    prompt_body: "p".into(),
                    agent_body: None,
                    output: "debug/raw/".into(),
                    context_paths: vec![],
                    tool_ids: vec![],
                    model_id: None,
                    inputs: vec![],
                    avatar: None,
                },
                PenguinSpec {
                    name: "Structure".into(),
                    kind: "process".into(),
                    display_name: "Structure".into(),
                    description: "d".into(),
                    prompt_body: "p".into(),
                    agent_body: None,
                    output: "debug/stage.md".into(),
                    context_paths: vec!["@artifact@".into()],
                    tool_ids: vec![],
                    model_id: None,
                    inputs: vec![],
                    avatar: None,
                },
                PenguinSpec {
                    name: "Deliver".into(),
                    kind: "deliver".into(),
                    display_name: "Deliver".into(),
                    description: "d".into(),
                    prompt_body: "p".into(),
                    agent_body: None,
                    output: "HANDOFF.md".into(),
                    context_paths: vec!["@artifact@".into()],
                    tool_ids: vec![],
                    model_id: None,
                    inputs: vec![],
                    avatar: None,
                },
            ],
            default_question: None,
            default_topic: None,
            workdir: Some("output".into()),
            delivery_title: None,
            delivery_note: None,
            delivery_root_rel: None,
            delivery_summary_note: None,
            prompt_tip: None,
        };
        normalize_draft(&mut draft);
        assert_eq!(draft.id, "rust-project-scaffolder-1-0");
        assert_eq!(
            draft.pipeline,
            vec!["ingest", "structure", "deliver"]
        );
        validate_draft(&draft).expect("normalized draft should validate");
    }
}
