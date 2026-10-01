//! Operator-facing form fields for horde runs (declared in agent frontmatter).

use crate::error::KowalskiError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One field in a pre-run operator form (`[[inputs]]` in `agents/<step>.md`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorInputField {
    pub id: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub label: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
}

/// Form shown before starting a horde run (usually the first pipeline step).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HordeRunFormSpec {
    pub step: String,
    pub display_name: Option<String>,
    pub inputs: Vec<OperatorInputField>,
}

/// Validate answers against a form spec; returns errors keyed by field id.
pub fn validate_form_answers(
    form: &HordeRunFormSpec,
    answers: &BTreeMap<String, String>,
) -> Result<(), KowalskiError> {
    let mut errs = Vec::new();
    for field in &form.inputs {
        let v = answers.get(&field.id).map(|s| s.trim()).unwrap_or("");
        if field.required && v.is_empty() {
            errs.push(format!("`{}` is required", field.label));
            continue;
        }
        if v.is_empty() {
            continue;
        }
        match field.field_type.as_str() {
            "url" if !v.starts_with("http://") && !v.starts_with("https://") => {
                errs.push(format!("`{}` must be a valid URL", field.label));
            }
            "path" => {
                let p = std::path::Path::new(v);
                if !p.is_dir() {
                    errs.push(format!(
                        "`{}` must be an existing directory (got: {})",
                        field.label, v
                    ));
                }
            }
            "choice" if !field.options.is_empty() && !field.options.iter().any(|o| o == v) => {
                errs.push(format!(
                    "`{}` must be one of: {}",
                    field.label,
                    field.options.join(", ")
                ));
            }
            _ => {}
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(KowalskiError::Validation(errs.join("; ")))
    }
}

/// Build a single prompt string from form answers (for horde run `source` / `prompt`).
pub fn answers_to_prompt(form: &HordeRunFormSpec, answers: &BTreeMap<String, String>) -> String {
    let mut lines = vec![format!(
        "# Operator input ({})",
        form.display_name.as_deref().unwrap_or(&form.step)
    )];
    for field in &form.inputs {
        let v = answers
            .get(&field.id)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .or(field.default.as_deref());
        if let Some(val) = v {
            lines.push(format!("**{}:** {}", field.label, val));
        }
    }
    lines.join("\n\n")
}

/// Parse `**Label:** value` lines from an operator prompt block built by [`answers_to_prompt`].
///
/// Keys are field labels; values may span multiple lines until the next `**Label:**` line.
pub fn parse_operator_answer_block(source: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut current_label: Option<String> = None;
    let mut current_value = String::new();

    let flush = |label: &mut Option<String>, value: &mut String, map: &mut BTreeMap<String, String>| {
        if let Some(l) = label.take() {
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty() {
                map.insert(l, trimmed);
            }
            value.clear();
        }
    };

    for line in source.lines() {
        if line.starts_with("**")
            && let Some(idx) = line.find(":**")
        {
            flush(&mut current_label, &mut current_value, &mut out);
            let label = line[2..idx].trim().to_string();
            let value = line[idx + 3..].trim();
            current_label = Some(label);
            if !value.is_empty() {
                current_value.push_str(value);
            }
            continue;
        }
        if current_label.is_some() {
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(line);
        }
    }
    flush(&mut current_label, &mut current_value, &mut out);
    out
}

/// Find an operator answer by field id (exact label match or label containing the id).
pub fn operator_answer<'a>(answers: &'a BTreeMap<String, String>, field_id: &str) -> Option<&'a str> {
    answers.get(field_id).map(|s| s.as_str()).or_else(|| {
        answers
            .iter()
            .find(|(label, _)| label.eq_ignore_ascii_case(field_id) || label.contains(field_id))
            .map(|(_, v)| v.as_str())
    })
}

/// Default ingest-stage form for Rust / greenfield project hordes.
pub fn default_ingest_form_fields() -> Vec<OperatorInputField> {
    vec![
        OperatorInputField {
            id: "project_name".into(),
            field_type: "text".into(),
            label: "Project name".into(),
            required: true,
            placeholder: Some("my-rust-service".into()),
            options: vec![],
            default: None,
        },
        OperatorInputField {
            id: "project_goals".into(),
            field_type: "textarea".into(),
            label: "Goals and constraints".into(),
            required: true,
            placeholder: Some(
                "e.g. CLI tool, async HTTP, SQLite, no cloud deps…".into(),
            ),
            options: vec![],
            default: None,
        },
        OperatorInputField {
            id: "repo_url".into(),
            field_type: "url".into(),
            label: "Existing repository URL (optional)".into(),
            required: false,
            placeholder: Some("https://github.com/org/repo".into()),
            options: vec![],
            default: None,
        },
        OperatorInputField {
            id: "crate_focus".into(),
            field_type: "choice".into(),
            label: "Primary project shape".into(),
            required: false,
            placeholder: None,
            options: vec![
                "cli".into(),
                "web-api".into(),
                "library".into(),
                "embedded".into(),
            ],
            default: Some("cli".into()),
        },
    ]
}

/// Longest run title, in characters.
const RUN_TITLE_MAX: usize = 80;

/// A short, human title for a run, so a list of runs reads as what was asked: the first answer
/// of an operator form (its first line, with "+N more" when it has more lines), a file name for
/// a path, a host for a URL, the first line of free text, or what started a trigger run.
pub fn run_title(prompt: &str, question: &str, source: Option<&str>) -> String {
    // the operator's own form answers say what the run is about; a horde's default question
    // (used when the form has no free-text question) would name every run the same
    // A typed request (no form) with words of its own, not just links, says what the run is
    // about better than the question, which from the UI is the horde's default.
    let is_link = |l: &str| l.starts_with("http://") || l.starts_with("https://") || l.starts_with('/') || l.starts_with("~/");
    let content_line = |t: &str| {
        t.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#') && !is_link(l)).map(str::to_string)
    };
    let typed_words = first_answer(prompt).is_none() && content_line(prompt).is_some();
    let text = if first_answer(prompt).is_some() || typed_words || question.trim().is_empty() { prompt } else { question };
    if let Some(kind) = source
        .and_then(|s| s.strip_prefix(crate::horde_trigger::TRIGGER_SOURCE_PREFIX))
        .and_then(|rest| rest.split(':').next())
    {
        match kind {
            "cron" => return "Scheduled run".to_string(),
            "webhook" => return "Webhook call".to_string(),
            _ => {} // a watched path: the file name below says what arrived
        }
    }
    let answer = first_answer(text);
    let lines: Vec<&str> = match answer.as_deref() {
        Some(answer) => answer.lines().map(str::trim).filter(|l| !l.is_empty()).collect(),
        None => {
            let lines: Vec<&str> = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("# Operator input"))
                .collect();
            // the first line with words, else the first line (a link becomes its short label)
            lines.iter().find(|l| !is_link(l)).or(lines.first()).copied().into_iter().collect()
        }
    };
    let Some(first) = lines.first() else {
        return "Untitled run".to_string();
    };
    let mut title = short_label(first);
    if lines.len() > 1 {
        title.push_str(&format!(" +{} more", lines.len() - 1));
    }
    title
}

/// The value of the first `**Label:** value` field of an operator block (it may span lines).
fn first_answer(text: &str) -> Option<String> {
    let mut value: Option<String> = None;
    for line in text.lines() {
        let t = line.trim_start();
        let is_label = t.starts_with("**") && t.contains(":**");
        match (&mut value, is_label) {
            (None, true) => {
                let rest = &t[t.find(":**")? + 3..];
                value = Some(rest.trim().to_string());
            }
            (Some(_), true) => break,
            (Some(v), false) => {
                v.push('\n');
                v.push_str(line);
            }
            (None, false) => {}
        }
    }
    value.filter(|v| !v.trim().is_empty())
}

/// One line as a label: markdown marks stripped, a path shown by its file name, a URL by its
/// host, cut to [`RUN_TITLE_MAX`] characters.
fn short_label(line: &str) -> String {
    let t = line.trim_start_matches(['#', '*', '>', '-', ' ']).trim().trim_matches('`');
    let t = if let Some(rest) = t.strip_prefix("https://").or_else(|| t.strip_prefix("http://")) {
        rest.split('/').next().unwrap_or(rest).to_string()
    } else if t.starts_with('/') || t.starts_with("~/") {
        t.rsplit('/').find(|p| !p.is_empty()).unwrap_or(t).to_string()
    } else {
        t.to_string()
    };
    match t.char_indices().nth(RUN_TITLE_MAX) {
        Some((cut, _)) => format!("{}…", t[..cut].trim_end()),
        None => t,
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn run_titles_read_as_what_was_asked() {
        let form = "# Operator input (Your questions)\n\n**Questions, one per line:** Who spent the most?\nHow many are active?\nPeople per city?\n\n**Tables:** orders";
        assert_eq!(run_title(form, form, Some(form)), "Who spent the most? +2 more");
        assert_eq!(run_title("", "/Users/me/inbox/acme-invoice.txt", Some("trigger:watch:folder-watcher")), "acme-invoice.txt");
        assert_eq!(run_title("", "anything", Some("trigger:cron:morning-brief")), "Scheduled run");
        let brief = "# Operator input (Fetch)\n\n**Pages to read:** https://news.ycombinator.com/\nhttps://tldr.tech/";
        assert_eq!(run_title(brief, brief, None), "news.ycombinator.com +1 more");
        assert_eq!(run_title("## Summarise this repo please", "", None), "Summarise this repo please");
        assert_eq!(run_title("", "", None), "Untitled run");
        let typed = "https://www.rust-lang.org/learn\nWhat are the main ways to learn Rust?";
        assert_eq!(run_title(typed, "What changed in the latest source?", None), "What are the main ways to learn Rust?", "typed words beat the default question");
        assert_eq!(run_title("https://www.rust-lang.org/learn", "What are the main ways to learn Rust?", None), "What are the main ways to learn Rust?", "a links-only request keeps the question");
        let brief = "# Operator input (Brief)\n\n**What should be checked?:** Is the release ready?";
        assert_eq!(run_title(brief, "Are the checks green?", None), "Is the release ready?", "form answers beat the default question");
        let long = "x".repeat(200);
        assert!(run_title(&long, "", None).chars().count() <= RUN_TITLE_MAX + 1);
    }
    use super::*;

    fn sample_form() -> HordeRunFormSpec {
        HordeRunFormSpec {
            step: "ingest".into(),
            display_name: Some("Project Input".into()),
            inputs: default_ingest_form_fields(),
        }
    }

    fn answers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let form = sample_form();
        let err = validate_form_answers(&form, &answers(&[("project_name", "demo")]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("Goals and constraints"), "got: {err}");
    }

    #[test]
    fn required_fields_present_passes() {
        let form = sample_form();
        let a = answers(&[("project_name", "demo"), ("project_goals", "cli tool")]);
        assert!(validate_form_answers(&form, &a).is_ok());
    }

    #[test]
    fn invalid_url_is_rejected() {
        let form = sample_form();
        let a = answers(&[
            ("project_name", "demo"),
            ("project_goals", "cli tool"),
            ("repo_url", "not-a-url"),
        ]);
        assert!(validate_form_answers(&form, &a).is_err());
    }

    #[test]
    fn choice_outside_options_is_rejected() {
        let form = sample_form();
        let a = answers(&[
            ("project_name", "demo"),
            ("project_goals", "cli tool"),
            ("crate_focus", "mainframe"),
        ]);
        assert!(validate_form_answers(&form, &a).is_err());
    }

    #[test]
    fn prompt_includes_answered_fields_and_skips_blanks() {
        let form = sample_form();
        let a = answers(&[("project_name", "demo"), ("project_goals", "cli tool")]);
        let prompt = answers_to_prompt(&form, &a);
        assert!(prompt.contains("# Operator input (Project Input)"));
        assert!(prompt.contains("**Project name:** demo"));
        assert!(prompt.contains("**Goals and constraints:** cli tool"));
        assert!(!prompt.contains("Existing repository URL"));
    }

    #[test]
    fn prompt_falls_back_to_field_default() {
        let form = sample_form();
        let a = answers(&[("project_name", "demo"), ("project_goals", "cli tool")]);
        let prompt = answers_to_prompt(&form, &a);
        // `crate_focus` has default "cli" and is unanswered → default is emitted.
        assert!(prompt.contains("**Primary project shape:** cli"));
    }

    #[test]
    fn path_field_requires_directory() {
        let form = HordeRunFormSpec {
            step: "ingest".into(),
            display_name: None,
            inputs: vec![OperatorInputField {
                id: "project_path".into(),
                field_type: "path".into(),
                label: "Project path".into(),
                required: true,
                placeholder: None,
                options: vec![],
                default: None,
            }],
        };
        let a = answers(&[("project_path", "/no/such/dir")]);
        assert!(validate_form_answers(&form, &a).is_err());
    }

    #[test]
    fn parse_operator_block_multiline() {
        let block = "# Operator input\n\n**Task specification:** line one\nline two\n\n**Project path:** /tmp\n";
        let m = parse_operator_answer_block(block);
        assert_eq!(
            m.get("Task specification").map(String::as_str),
            Some("line one\nline two")
        );
        assert_eq!(m.get("Project path").map(String::as_str), Some("/tmp"));
    }
}
