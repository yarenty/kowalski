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
