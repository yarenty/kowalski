//! Write a born horde directory from a validated [`RookeryDraft`].

use crate::error::KowalskiError;
use crate::horde_graph::{inbound_predecessors, resolve_execution_graph, should_persist_edges};
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
    let has_prompt = !penguin.prompt_body.trim().is_empty();
    if has_prompt {
        fs::write(horde_root.join(&prompt_rel), &penguin.prompt_body)?;
    }

    let capability = penguin
        .capability
        .clone()
        .unwrap_or_else(|| format!("{}.{}", prefix, penguin.kind));
    let default_agent_id = penguin
        .default_agent_id
        .clone()
        .unwrap_or_else(|| format!("{}-{}", prefix.replace('.', "-"), penguin.kind));
    let context_paths = effective_context_paths(draft, penguin);

    let mut fm = String::new();
    fm.push_str("---\n");
    fm.push_str(&format!("name = \"{}\"\n", penguin.name));
    fm.push_str(&format!("kind = \"{}\"\n", penguin.kind));
    if let Some(avatar) = penguin.avatar.as_deref().filter(|s| !s.trim().is_empty()) {
        fm.push_str(&format!("avatar = \"{}\"\n", escape_toml_str(avatar)));
    }
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
    if has_prompt {
        fm.push_str(&format!("prompt_file = \"{prompt_rel}\"\n"));
    }
    let output = effective_output(draft, penguin);
    fm.push_str(&format!("output = \"{}\"\n", escape_toml_str(&output)));
    if let Some(model_id) = penguin.model_id.as_deref().filter(|s| !s.trim().is_empty()) {
        fm.push_str(&format!("model_id = \"{}\"\n", escape_toml_str(model_id)));
    }
    if let Some(isolation) = penguin.isolation.as_deref().filter(|s| !s.trim().is_empty()) {
        fm.push_str(&format!("isolation = \"{}\"\n", escape_toml_str(isolation)));
    }
    if let Some(cmd) = penguin.verify_command.as_deref() {
        fm.push_str(&format!("verify_command = \"{}\"\n", escape_toml_str(cmd)));
    }
    if let Some(cwd) = penguin.verify_cwd.as_deref() {
        fm.push_str(&format!("verify_cwd = \"{}\"\n", escape_toml_str(cwd)));
    }
    if let Some(mode) = penguin.apply_mode.as_deref() {
        fm.push_str(&format!("apply_mode = \"{}\"\n", escape_toml_str(mode)));
    }
    if let Some(title) = penguin.normalize_doc_title.as_deref() {
        fm.push_str(&format!(
            "normalize_doc_title = \"{}\"\n",
            escape_toml_str(title)
        ));
    }
    if !penguin.normalize_sections.is_empty() {
        fm.push_str(&format!(
            "normalize_sections = [{}]\n",
            toml_string_array(&penguin.normalize_sections)
        ));
    }
    if let Some(fallback) = penguin.normalize_fallback.as_deref() {
        fm.push_str(&format!(
            "normalize_fallback = \"{}\"\n",
            escape_toml_str(fallback)
        ));
    }
    if !penguin.normalize_fallback_sections.is_empty() {
        fm.push_str(&format!(
            "normalize_fallback_sections = [{}]\n",
            toml_string_array(&penguin.normalize_fallback_sections)
        ));
    }
    if !context_paths.is_empty() {
        fm.push_str(&format!(
            "context_paths = [{}]\n",
            toml_string_array(&context_paths)
        ));
    }
    if !penguin.tool_ids.is_empty() {
        fm.push_str(&format!(
            "tool_ids = [{}]\n",
            toml_string_array(&penguin.tool_ids)
        ));
    }
    // `[[inputs]]` array-of-tables blocks must come last: any bare key emitted after
    // them would parse as a key of the final inputs table, not the agent frontmatter.
    for input in &penguin.inputs {
        write_input_field(&mut fm, input);
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

fn toml_string_array(items: &[String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{}\"", escape_toml_str(s)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn escape_toml_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
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
        fm.push_str(&format!(
            "options = [{}]\n",
            toml_string_array(&input.options)
        ));
    }
}

/// The agent-file body the writer emits when a penguin declares none.
pub(crate) fn default_agent_body(penguin: &PenguinSpec) -> String {
    format!(
        "# {}\n\n{}\n",
        penguin.display_name, penguin.description
    )
}

/// Context paths the writer would emit for this penguin: the declared paths, or the
/// `@artifact@` default for non-source graph steps. Shared with the manifest exporter
/// so exported manifests are self-contained.
pub(crate) fn effective_context_paths(draft: &RookeryDraft, penguin: &PenguinSpec) -> Vec<String> {
    if !penguin.context_paths.is_empty() {
        penguin.context_paths.clone()
    } else if graph_step_is_source(draft, &penguin.name) {
        vec![]
    } else {
        vec!["@artifact@".to_string()]
    }
}

fn graph_step_is_source(draft: &RookeryDraft, step: &str) -> bool {
    let edge_slice = if draft.edges.is_empty() {
        None
    } else {
        Some(draft.edges.as_slice())
    };
    resolve_execution_graph(&draft.pipeline, edge_slice)
        .map(|g| inbound_predecessors(&g.edges, step).is_empty())
        .unwrap_or_else(|_| draft.pipeline.first().is_some_and(|s| s == step))
}

/// Horde.md field values after applying the writer's defaults. Single owner of those
/// defaults — the renderer and the manifest exporter both resolve through here.
#[derive(Debug, Clone)]
pub struct ResolvedHordeFields {
    pub capability_prefix: String,
    pub default_question: String,
    pub default_topic: String,
    pub workdir: String,
    pub delivery_title: String,
    pub delivery_note: String,
    pub delivery_root_rel: String,
    pub delivery_summary_note: String,
    /// Empty string when unset (omitted from horde.md).
    pub prompt_tip: String,
}

/// Resolve every optional horde.md field to the value [`write_horde_tree`] would emit.
pub fn resolve_horde_fields(draft: &RookeryDraft) -> ResolvedHordeFields {
    let delivery_root_rel = draft
        .delivery_root_rel
        .clone()
        .unwrap_or_else(|| last_penguin_output(draft).unwrap_or_else(|| "HANDOFF.md".into()));
    ResolvedHordeFields {
        capability_prefix: capability_prefix(draft),
        default_question: draft
            .default_question
            .clone()
            .unwrap_or_else(|| "What should we do with the latest output?".into()),
        default_topic: draft
            .default_topic
            .clone()
            .unwrap_or_else(|| "federation".into()),
        workdir: draft.workdir.clone().unwrap_or_else(|| "output".into()),
        delivery_title: draft
            .delivery_title
            .clone()
            .unwrap_or_else(|| "Delivery".into()),
        delivery_note: draft.delivery_note.clone().unwrap_or_else(|| {
            format!(
                "When the run finishes, open **`workdir/{delivery_root_rel}`**. Intermediates live under **`workdir/debug/`** per agent `output` paths."
            )
        }),
        delivery_summary_note: draft
            .delivery_summary_note
            .clone()
            .unwrap_or_else(|| draft.description.clone()),
        delivery_root_rel,
        prompt_tip: draft.prompt_tip.clone().unwrap_or_default(),
    }
}

fn render_horde_md(draft: &RookeryDraft) -> String {
    let ResolvedHordeFields {
        capability_prefix: prefix,
        default_question,
        default_topic,
        workdir,
        delivery_title,
        delivery_note,
        delivery_root_rel,
        delivery_summary_note: delivery_summary,
        prompt_tip,
    } = resolve_horde_fields(draft);
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
    if let Some(version) = draft.version.as_deref().filter(|s| !s.trim().is_empty()) {
        out.push_str(&format!("version = \"{}\"\n", escape_toml_str(version)));
    }
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
    if should_persist_edges(&draft.pipeline, &draft.edges) {
        for edge in &draft.edges {
            out.push_str(&format!(
                "\n[[edges]]\nfrom = \"{}\"\nto = \"{}\"\n",
                edge.from, edge.to
            ));
            if let Some(when) = &edge.when {
                out.push_str(&format!("when = \"{}\"\n", when));
            }
            if let Some(max_loops) = edge.max_loops {
                out.push_str(&format!("max_loops = {}\n", max_loops));
            }
        }
    }
    for trigger in &draft.triggers {
        out.push_str(&render_trigger_toml(trigger));
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
    if should_persist_edges(&draft.pipeline, &draft.edges) {
        out.push_str(
            "DAG pipeline (`pipeline` is a topological order; `[[edges]]` define scheduling deps). \
Parallel branches run sequentially per process in 1.5.0 MVP.\n\n```\n",
        );
    } else {
        out.push_str("Linear pipeline:\n\n```\n");
    }
    out.push_str(&draft.pipeline.join(" -> "));
    out.push_str("\n```\n");
    if should_persist_edges(&draft.pipeline, &draft.edges) {
        out.push_str("\n### Edges\n\n");
        for edge in &draft.edges {
            out.push_str(&format!("- `{}` → `{}`\n", edge.from, edge.to));
        }
    }
    out
}

/// One `[[triggers]]` frontmatter block. Every stored field is emitted (defaults included)
/// so a round-trip through the parser reproduces the draft exactly.
fn render_trigger_toml(trigger: &crate::horde_trigger::HordeTrigger) -> String {
    let mut out = String::from("\n[[triggers]]\n");
    if let Some(cron) = &trigger.cron {
        out.push_str(&format!("cron = \"{}\"\n", escape_toml_str(cron)));
    }
    if let Some(watch) = &trigger.watch {
        let events = watch
            .events
            .iter()
            .map(|e| format!("\"{}\"", escape_toml_str(e)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "watch = {{ path = \"{}\", events = [{}], debounce_ms = {} }}\n",
            escape_toml_str(&watch.path),
            events,
            watch.debounce_ms
        ));
    }
    if let Some(webhook) = &trigger.webhook {
        out.push_str(&format!(
            "webhook = {{ route = \"{}\" }}\n",
            escape_toml_str(&webhook.route)
        ));
    }
    out.push_str(&format!("enabled = {}\n", trigger.enabled));
    out.push_str(&format!(
        "overlap = \"{}\"\n",
        escape_toml_str(&trigger.overlap)
    ));
    if !trigger.input.is_empty() {
        let inner = trigger
            .input
            .iter()
            .map(|(k, v)| format!("\"{}\" = \"{}\"", escape_toml_str(k), escape_toml_str(v)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("input = {{ {inner} }}\n"));
    }
    if let Some(prompt) = &trigger.prompt {
        out.push_str(&format!("prompt = \"{}\"\n", escape_toml_str(prompt)));
    }
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
