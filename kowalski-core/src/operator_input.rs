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

#[cfg(test)]
mod tests {
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
