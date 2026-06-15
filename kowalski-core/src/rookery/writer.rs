//! Write a born horde directory from a validated [`RookeryDraft`].

use crate::error::KowalskiError;
use crate::rookery::types::{HordeBirthSpec, PenguinSpec, RookeryDraft};
use crate::operator_input::OperatorInputField;
use crate::rookery::normalize::{default_output_for_penguin, output_looks_invalid};
use crate::rookery::validate::{validate_draft, validate_horde_id};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve `<output_root>/<horde_id>/`.
pub fn horde_root_path(output_root: &Path, horde_id: &str) -> Result<PathBuf, KowalskiError> {
    validate_horde_id(horde_id)?;
    let root = output_root.join(horde_id);
    if root.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(KowalskiError::Validation(
            "output_root must not escape via `..`".into(),
        ));
    }
    Ok(root)
}

/// Write `horde.md`, `agents/`, `prompts/`, `README.md`, and `AGENTS.md` under the horde root.
pub fn write_horde_tree(output_root: &Path, spec: &HordeBirthSpec) -> Result<PathBuf, KowalskiError> {
    validate_draft(&spec.draft)?;
    let horde_root = horde_root_path(output_root, &spec.draft.id)?;
    if horde_root.exists() {
        if !spec.overwrite {
            return Err(KowalskiError::Validation(format!(
                "horde directory already exists: {} (pass overwrite=true to replace)",
                horde_root.display()
            )));
        }
        fs::remove_dir_all(&horde_root)?;
    }
    fs::create_dir_all(horde_root.join("agents"))?;
    fs::create_dir_all(horde_root.join("prompts"))?;

    fs::write(horde_root.join("horde.md"), render_horde_md(&spec.draft))?;
    for penguin in &spec.draft.penguins {
        write_penguin_files(&horde_root, &spec.draft, penguin)?;
    }
    fs::write(horde_root.join("README.md"), render_readme(&spec.draft))?;
    fs::write(horde_root.join("AGENTS.md"), render_agents_md(&spec.draft))?;

    Ok(horde_root)
}

fn capability_prefix(draft: &RookeryDraft) -> String {
    draft
        .capability_prefix
        .clone()
        .unwrap_or_else(|| draft.id.clone())
}

fn write_penguin_files(
    horde_root: &Path,
    draft: &RookeryDraft,
    penguin: &PenguinSpec,
) -> Result<(), KowalskiError> {
    let prefix = capability_prefix(draft);
    let prompt_rel = format!("prompts/{}.md", penguin.name);
    fs::write(horde_root.join(&prompt_rel), &penguin.prompt_body)?;

    let capability = format!("{}.{}", prefix, penguin.kind);
    let default_agent_id = format!(
        "{}-{}",
        prefix.replace('.', "-"),
        penguin.kind
    );
    let context_paths = if penguin.context_paths.is_empty() {
        if draft.pipeline.first() == Some(&penguin.name) {
            vec![]
        } else {
            vec!["@artifact@".to_string()]
        }
    } else {
        penguin.context_paths.clone()
    };

    let mut fm = String::new();
    fm.push_str("---\n");
    fm.push_str(&format!("name = \"{}\"\n", penguin.name));
    fm.push_str(&format!("kind = \"{}\"\n", penguin.kind));
    fm.push_str(&format!("capability = \"{capability}\"\n"));
    fm.push_str(&format!("default_agent_id = \"{default_agent_id}\"\n"));
    fm.push_str(&format!(
        "display_name = \"{}\"\n",
        escape_toml_str(&penguin.display_name)
    ));
    fm.push_str(&format!(
        "description = \"{}\"\n",
        escape_toml_str(&penguin.description)
    ));
    fm.push_str(&format!("prompt_file = \"{prompt_rel}\"\n"));
    let output = effective_output(draft, penguin);
    fm.push_str(&format!("output = \"{}\"\n", escape_toml_str(&output)));
    if !penguin.inputs.is_empty() {
        for input in &penguin.inputs {
            write_input_field(&mut fm, input);
        }
    }
    if !context_paths.is_empty() {
        let inner = context_paths
            .iter()
            .map(|s| format!("\"{}\"", escape_toml_str(s)))
            .collect::<Vec<_>>()
            .join(", ");
        fm.push_str(&format!("context_paths = [{inner}]\n"));
    }
    fm.push_str("---\n\n");

    let default_body = default_agent_body(penguin);
    let agent_body = penguin
        .agent_body
        .as_deref()
        .unwrap_or(default_body.as_str());
    fs::write(
        horde_root.join("agents").join(format!("{}.md", penguin.name)),
        format!("{fm}{agent_body}"),
    )?;
    Ok(())
}

fn escape_toml_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn effective_output(draft: &RookeryDraft, penguin: &PenguinSpec) -> String {
    if output_looks_invalid(&penguin.output) {
        let is_first = draft.pipeline.first() == Some(&penguin.name);
        let is_last = draft.pipeline.last() == Some(&penguin.name);
        default_output_for_penguin(draft.delivery_root_rel.as_deref(), penguin, is_first, is_last)
    } else {
        penguin.output.clone()
    }
}

fn write_input_field(fm: &mut String, input: &OperatorInputField) {
    fm.push_str("[[inputs]]\n");
    fm.push_str(&format!("id = \"{}\"\n", escape_toml_str(&input.id)));
    fm.push_str(&format!("type = \"{}\"\n", escape_toml_str(&input.field_type)));
    fm.push_str(&format!("label = \"{}\"\n", escape_toml_str(&input.label)));
    if input.required {
        fm.push_str("required = true\n");
    }
    if let Some(p) = &input.placeholder {
        fm.push_str(&format!(
            "placeholder = \"{}\"\n",
            escape_toml_str(p)
        ));
    }
    if let Some(d) = &input.default {
        fm.push_str(&format!("default = \"{}\"\n", escape_toml_str(d)));
    }
    if !input.options.is_empty() {
        let inner = input
            .options
            .iter()
            .map(|o| format!("\"{}\"", escape_toml_str(o)))
            .collect::<Vec<_>>()
            .join(", ");
        fm.push_str(&format!("options = [{inner}]\n"));
    }
}

fn default_agent_body(penguin: &PenguinSpec) -> String {
    format!(
        "# {}\n\n{}\n",
        penguin.display_name, penguin.description
    )
}

fn render_horde_md(draft: &RookeryDraft) -> String {
    let workdir = draft.workdir.as_deref().unwrap_or("output");
    let delivery_root_rel = draft
        .delivery_root_rel
        .clone()
        .unwrap_or_else(|| last_penguin_output(draft).unwrap_or_else(|| "HANDOFF.md".into()));
    let default_question = draft
        .default_question
        .clone()
        .unwrap_or_else(|| "What should we do with the latest output?".into());
    let default_topic = draft
        .default_topic
        .clone()
        .unwrap_or_else(|| "federation".into());
    let delivery_title = draft
        .delivery_title
        .clone()
        .unwrap_or_else(|| "Delivery".into());
    let delivery_note = draft.delivery_note.clone().unwrap_or_else(|| {
        format!(
            "When the run finishes, open **`workdir/{delivery_root_rel}`**. Intermediates live under **`workdir/debug/`** per agent `output` paths."
        )
    });
    let delivery_summary = draft
        .delivery_summary_note
        .clone()
        .unwrap_or_else(|| draft.description.clone());
    let prompt_tip = draft.prompt_tip.clone().unwrap_or_default();
    let prefix = capability_prefix(draft);
    let pipeline_toml: String = draft
        .pipeline
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("id = \"{}\"\n", draft.id));
    out.push_str(&format!(
        "display_name = \"{}\"\n",
        escape_toml_str(&draft.display_name)
    ));
    out.push_str(&format!(
        "description = \"{}\"\n",
        escape_toml_str(&draft.description)
    ));
    out.push_str(&format!("capability_prefix = \"{prefix}\"\n"));
    out.push_str(&format!("pipeline = [{pipeline_toml}]\n"));
    out.push_str(&format!(
        "default_question = \"{}\"\n",
        escape_toml_str(&default_question)
    ));
    out.push_str(&format!("default_topic = \"{default_topic}\"\n"));
    out.push_str("artifacts_root = \".\"\n");
    out.push_str(&format!("workdir = \"{workdir}\"\n"));
    out.push_str(&format!(
        "delivery_title = \"{}\"\n",
        escape_toml_str(&delivery_title)
    ));
    out.push_str(&format!(
        "delivery_note = \"{}\"\n",
        escape_toml_str(&delivery_note)
    ));
    out.push_str(&format!("delivery_root_rel = \"{delivery_root_rel}\"\n"));
    out.push_str(&format!(
        "delivery_summary_note = \"{}\"\n",
        escape_toml_str(&delivery_summary)
    ));
    if !prompt_tip.is_empty() {
        out.push_str(&format!(
            "prompt_tip = \"{}\"\n",
            escape_toml_str(&prompt_tip)
        ));
    }
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", draft.display_name));
    out.push_str(&format!("{}\n\n", draft.description));
    out.push_str("## Sub-agents (penguins)\n\n");
    for step in &draft.pipeline {
        if let Some(p) = draft.penguins.iter().find(|x| &x.name == step) {
            out.push_str(&format!(
                "- `{}` ({}): {}\n",
                p.name, p.kind, p.description
            ));
        }
    }
    out.push_str("\n## Orchestration model\n\n");
    out.push_str("Linear pipeline (1.3.0):\n\n```\n");
    out.push_str(&draft.pipeline.join(" -> "));
    out.push_str("\n```\n");
    out
}

fn last_penguin_output(draft: &RookeryDraft) -> Option<String> {
    let last = draft.pipeline.last()?;
    draft
        .penguins
        .iter()
        .find(|p| &p.name == last)
        .map(|p| p.output.clone())
}

fn render_readme(draft: &RookeryDraft) -> String {
    let workdir = draft.workdir.as_deref().unwrap_or("output");
    format!(
        "# {}\n\n{}\n\n## Quick start\n\n```bash\ncargo run -p kowalski-cli -- agent-app validate --path .\ncargo run -p kowalski-cli -- agent-app run --path . \"your source text or URL\"\n```\n\nArtifacts default to **`{workdir}/`** (see `horde.md`).\n\nBorn with **Rookery** (Kowalski 1.3.0).\n",
        draft.display_name, draft.description
    )
}

fn render_agents_md(draft: &RookeryDraft) -> String {
    format!(
        "# {} — operator guide\n\n> Horde generated by **Rookery**. Pipeline: `{}`.\n\n## Validate\n\n```bash\ncargo run -p kowalski-cli -- agent-app validate --path .\n```\n\n## Layout\n\n- `horde.md` — manifest\n- `agents/*.md` — one penguin per pipeline step\n- `prompts/*.md` — LLM prompts\n- `workdir` — runtime output (see `horde.md`)\n",
        draft.display_name,
        draft.pipeline.join(" → ")
    )
}
