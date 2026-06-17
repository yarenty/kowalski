//! Parse `RookeryDraft` from builder output (TOML preferred, JSON accepted).

use crate::error::KowalskiError;
use crate::horde_graph::HordeEdge;
use crate::rookery::types::{PenguinSpec, RookeryDraft};
use serde::Deserialize;
use serde_json::{Map, Value, json};

const STRING_FROM_OBJECT_KEYS: &[&str] = &[
    "text",
    "summary",
    "description",
    "content",
    "body",
    "value",
    "path",
    "output",
    "markdown",
    "prompt",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::rookery) enum DraftBlockFormat {
    Toml,
    Json,
}

/// Lenient schema: missing fields are filled with sensible defaults before validation.
#[derive(Debug, Deserialize)]
struct LenientDraft {
    id: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    pipeline: Option<Vec<LenientPipelineStep>>,
    #[serde(default)]
    edges: Vec<HordeEdge>,
    penguins: Option<Vec<LenientPenguin>>,
    steps: Option<Vec<LenientPenguin>>,
    #[serde(default)]
    capability_prefix: Option<String>,
    #[serde(default)]
    default_question: Option<String>,
    #[serde(default)]
    default_topic: Option<String>,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    delivery_title: Option<String>,
    #[serde(default)]
    delivery_note: Option<String>,
    #[serde(default)]
    delivery_root_rel: Option<String>,
    #[serde(default)]
    delivery_summary_note: Option<String>,
    #[serde(default)]
    prompt_tip: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LenientPipelineStep {
    Name(String),
    Object { name: String },
}

#[derive(Debug, Deserialize, Default)]
struct LenientPenguin {
    name: Option<String>,
    kind: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    prompt_body: Option<String>,
    #[serde(alias = "prompt")]
    prompt: Option<String>,
    #[serde(alias = "output_path")]
    output_path: Option<String>,
    output: Option<String>,
    #[serde(default)]
    agent_body: Option<String>,
    #[serde(default)]
    context_paths: Option<Vec<String>>,
    #[serde(default)]
    tool_ids: Option<Vec<String>>,
    #[serde(default)]
    model_id: Option<String>,
    #[serde(default)]
    inputs: Vec<crate::operator_input::OperatorInputField>,
    #[serde(default)]
    avatar: Option<String>,
}

/// Extract draft body from a fenced block (TOML preferred) or raw `{...}` / TOML document.
pub fn extract_draft_block(text: &str) -> Option<(String, DraftBlockFormat)> {
    let trimmed = text.trim();
    for (marker, fmt) in [
        ("```toml", DraftBlockFormat::Toml),
        ("```TOML", DraftBlockFormat::Toml),
        ("```yaml", DraftBlockFormat::Toml),
        ("```yml", DraftBlockFormat::Toml),
        ("```json", DraftBlockFormat::Json),
        ("```JSON", DraftBlockFormat::Json),
        ("```", DraftBlockFormat::Toml),
    ] {
        if let Some(start) = trimmed.find(marker) {
            let rest = &trimmed[start + marker.len()..];
            let rest = rest.strip_prefix('\n').unwrap_or(rest);
            if let Some(end) = rest.find("```") {
                let inner = rest[..end].trim();
                if !inner.is_empty() {
                    let format = if inner.starts_with('{') {
                        DraftBlockFormat::Json
                    } else {
                        fmt
                    };
                    return Some((inner.to_string(), format));
                }
            }
        }
    }
    if trimmed.starts_with('{') {
        return Some((trimmed.to_string(), DraftBlockFormat::Json));
    }
    if trimmed.starts_with("id ") || trimmed.starts_with("id=") || trimmed.contains("[[penguins]]")
    {
        return Some((trimmed.to_string(), DraftBlockFormat::Toml));
    }
    None
}

/// Extract JSON from a fenced code block or raw `{...}` payload (legacy).
pub fn extract_json_block(text: &str) -> Option<String> {
    extract_draft_block(text).and_then(|(body, fmt)| {
        if fmt == DraftBlockFormat::Json {
            Some(body)
        } else {
            None
        }
    })
}

fn infer_kind(name: &str) -> String {
    let n = name.to_lowercase();
    for k in ["ingest", "deliver", "ask", "lint", "compile", "process"] {
        if n == k || n.contains(k) {
            return k.to_string();
        }
    }
    "process".to_string()
}

fn title_from_step_name(name: &str) -> String {
    name.split('-')
        .filter(|s| !s.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn lenient_penguin_to_spec(p: LenientPenguin, index: usize) -> PenguinSpec {
    let name = p
        .name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("step-{}", index + 1));
    let display_name = p
        .display_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| title_from_step_name(&name));
    let description = p
        .description
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| display_name.clone());
    let prompt_body = p
        .prompt_body
        .or(p.prompt)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "You are the **{}** stage (`{}`).\n\n{}\n",
                display_name, name, description
            )
        });
    let output = p
        .output
        .or(p.output_path)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_default();
    let kind = p
        .kind
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| infer_kind(&name));
    PenguinSpec {
        name,
        kind,
        display_name,
        description,
        prompt_body,
        agent_body: p.agent_body,
        output,
        context_paths: p.context_paths.unwrap_or_default(),
        tool_ids: p.tool_ids.unwrap_or_default(),
        model_id: p.model_id,
        inputs: p.inputs,
        avatar: p.avatar,
    }
}

fn lenient_to_draft(l: LenientDraft) -> Result<RookeryDraft, KowalskiError> {
    let penguins_raw = l
        .penguins
        .or(l.steps)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| KowalskiError::Validation("draft missing penguins[] or steps[]".into()))?;

    let penguins: Vec<PenguinSpec> = penguins_raw
        .into_iter()
        .enumerate()
        .map(|(i, p)| lenient_penguin_to_spec(p, i))
        .collect();

    let pipeline: Vec<String> = if let Some(pipe) = l.pipeline {
        pipe.into_iter()
            .map(|step| match step {
                LenientPipelineStep::Name(s) => s,
                LenientPipelineStep::Object { name } => name,
            })
            .collect()
    } else {
        penguins.iter().map(|p| p.name.clone()).collect()
    };

    if pipeline.is_empty() {
        return Err(KowalskiError::Validation(
            "draft pipeline must not be empty".into(),
        ));
    }

    let id = l
        .id
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| KowalskiError::Validation("draft missing id".into()))?;
    let display_name = l
        .display_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| title_from_step_name(&id));

    Ok(RookeryDraft {
        id,
        display_name,
        description: l
            .description
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Born from Rookery interview.".into()),
        capability_prefix: l.capability_prefix,
        pipeline,
        edges: l.edges,
        penguins,
        default_question: l.default_question,
        default_topic: l.default_topic,
        workdir: l.workdir.or(Some("output".into())),
        delivery_title: l.delivery_title,
        delivery_note: l.delivery_note,
        delivery_root_rel: l.delivery_root_rel,
        delivery_summary_note: l.delivery_summary_note,
        prompt_tip: l.prompt_tip,
    })
}

fn parse_lenient_toml(text: &str) -> Result<RookeryDraft, KowalskiError> {
    let l: LenientDraft = toml::from_str(text).map_err(|e| {
        KowalskiError::Validation(format!("invalid RookeryDraft TOML: {e}"))
    })?;
    lenient_to_draft(l)
}

fn parse_lenient_json_value(value: Value) -> Result<RookeryDraft, KowalskiError> {
    let coerced = coerce_draft_value(value);
    let l: LenientDraft = serde_json::from_value(coerced).map_err(|e| {
        KowalskiError::Validation(format!("invalid RookeryDraft JSON: {e}"))
    })?;
    lenient_to_draft(l)
}

fn string_from_value(v: Value) -> Value {
    match v {
        Value::String(s) => Value::String(s),
        Value::Number(n) => Value::String(n.to_string()),
        Value::Bool(b) => Value::String(b.to_string()),
        Value::Null => Value::String(String::new()),
        Value::Array(items) => {
            if items.iter().all(|x| x.is_string()) {
                Value::String(
                    items
                        .into_iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            } else {
                Value::String(
                    serde_json::to_string(&Value::Array(items)).unwrap_or_default(),
                )
            }
        }
        Value::Object(map) => {
            for key in STRING_FROM_OBJECT_KEYS {
                if let Some(s) = map.get(*key).and_then(Value::as_str) {
                    return Value::String(s.to_string());
                }
            }
            Value::String(serde_json::to_string(&Value::Object(map)).unwrap_or_default())
        }
    }
}

fn coerce_string_field(obj: &mut Map<String, Value>, key: &str) {
    if let Some(v) = obj.remove(key) {
        if !matches!(v, Value::String(_)) {
            obj.insert(key.to_string(), string_from_value(v));
        } else {
            obj.insert(key.to_string(), v);
        }
    }
}

fn coerce_optional_string_field(obj: &mut Map<String, Value>, key: &str) {
    if let Some(v) = obj.remove(key) {
        let coerced = match v {
            Value::Null => return,
            Value::String(_) => v,
            other => string_from_value(other),
        };
        obj.insert(key.to_string(), coerced);
    }
}

fn coerce_string_array_field(obj: &mut Map<String, Value>, key: &str) {
    let Some(v) = obj.remove(key) else {
        return;
    };
    let arr = match v {
        Value::Array(a) => a,
        Value::String(s) => vec![Value::String(s)],
        other => vec![string_from_value(other)],
    };
    let strings: Vec<Value> = arr
        .into_iter()
        .map(|item| match item {
            Value::String(s) => Value::String(s),
            Value::Object(o) => {
                for k in ["id", "name", "tool_id", "tool"] {
                    if let Some(s) = o.get(k).and_then(Value::as_str) {
                        return Value::String(s.to_string());
                    }
                }
                string_from_value(Value::Object(o))
            }
            other => string_from_value(other),
        })
        .collect();
    obj.insert(key.to_string(), Value::Array(strings));
}

fn coerce_edges_field(obj: &mut Map<String, Value>) {
    let Some(edges_val) = obj.get_mut("edges") else {
        return;
    };
    let Value::Array(items) = edges_val else {
        return;
    };
    let mut out = Vec::new();
    for item in items {
        match item {
            Value::Object(edge) => {
                let from = edge
                    .get("from")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                let to = edge
                    .get("to")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                if let (Some(from), Some(to)) = (from, to) {
                    out.push(json!({ "from": from, "to": to }));
                }
            }
            Value::String(s) if s.contains("->") => {
                let mut parts = s.split("->");
                if let (Some(from), Some(to)) = (parts.next(), parts.next()) {
                    let from = from.trim();
                    let to = to.trim();
                    if !from.is_empty() && !to.is_empty() {
                        out.push(json!({ "from": from, "to": to }));
                    }
                }
            }
            _ => {}
        }
    }
    *edges_val = Value::Array(out);
}

fn coerce_pipeline_field(obj: &mut Map<String, Value>) {
    let Some(v) = obj.remove("pipeline") else {
        return;
    };
    let arr = match v {
        Value::Array(a) => a,
        Value::String(s) => vec![Value::String(s)],
        other => vec![string_from_value(other)],
    };
    let steps: Vec<Value> = arr
        .into_iter()
        .map(|item| match item {
            Value::String(s) => Value::String(s),
            Value::Object(o) => {
                for k in ["name", "step", "id", "key"] {
                    if let Some(s) = o.get(k).and_then(Value::as_str) {
                        return Value::String(s.to_string());
                    }
                }
                string_from_value(Value::Object(o))
            }
            other => string_from_value(other),
        })
        .collect();
    obj.insert("pipeline".to_string(), Value::Array(steps));
}

fn fill_penguin_defaults(obj: &mut Map<String, Value>, index: usize) {
    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("step-{}", index + 1));
    obj.insert("name".to_string(), Value::String(name.clone()));

    if !obj.contains_key("kind") {
        obj.insert(
            "kind".to_string(),
            Value::String(infer_kind(&name)),
        );
    }
    if !obj.contains_key("display_name") {
        obj.insert(
            "display_name".to_string(),
            Value::String(title_from_step_name(&name)),
        );
    }
    if !obj.contains_key("description") {
        let dn = obj
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or(&name)
            .to_string();
        obj.insert("description".to_string(), Value::String(dn));
    }
    if !obj.contains_key("prompt_body") {
        let desc = obj
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        obj.insert(
            "prompt_body".to_string(),
            Value::String(if desc.is_empty() {
                format!("You are the **{name}** stage.\n")
            } else {
                format!("You are the **{name}** stage.\n\n{desc}\n")
            }),
        );
    }
    if !obj.contains_key("output") {
        obj.insert("output".to_string(), Value::String(String::new()));
    }
}

fn coerce_penguin_object(obj: &mut Map<String, Value>, index: usize) {
    if !obj.contains_key("prompt_body") {
        for alt in ["prompt", "prompt_markdown", "system_prompt", "instructions"] {
            if let Some(v) = obj.remove(alt) {
                obj.insert(
                    "prompt_body".to_string(),
                    if matches!(v, Value::String(_)) {
                        v
                    } else {
                        string_from_value(v)
                    },
                );
                break;
            }
        }
    }
    fill_penguin_defaults(obj, index);
    if let Some(v) = obj.remove("output") {
        let coerced = match v {
            Value::String(_) => v,
            other => string_from_value(other),
        };
        obj.insert("output".to_string(), coerced);
    } else if let Some(v) = obj.remove("output_path") {
        obj.insert(
            "output".to_string(),
            if matches!(v, Value::String(_)) {
                v
            } else {
                string_from_value(v)
            },
        );
    }
    if !obj.contains_key("tool_ids")
        && let Some(v) = obj.remove("tools")
    {
        obj.insert("tool_ids".to_string(), v);
    }
    for key in [
        "name",
        "kind",
        "display_name",
        "description",
        "prompt_body",
        "agent_body",
        "model_id",
    ] {
        coerce_string_field(obj, key);
    }
    coerce_optional_string_field(obj, "agent_body");
    coerce_optional_string_field(obj, "model_id");
    coerce_string_array_field(obj, "context_paths");
    coerce_string_array_field(obj, "tool_ids");
}

fn coerce_draft_value(mut root: Value) -> Value {
    let Value::Object(ref mut obj) = root else {
        return root;
    };

    for key in [
        "id",
        "display_name",
        "description",
        "capability_prefix",
        "default_question",
        "default_topic",
        "workdir",
        "delivery_title",
        "delivery_note",
        "delivery_root_rel",
        "delivery_summary_note",
        "prompt_tip",
    ] {
        coerce_string_field(obj, key);
    }
    coerce_optional_string_field(obj, "capability_prefix");
    coerce_pipeline_field(obj);
    coerce_edges_field(obj);

    if !obj.contains_key("penguins")
        && let Some(steps) = obj.remove("steps")
    {
        obj.insert("penguins".to_string(), steps);
    }

    if !obj.contains_key("id")
        && let Some(display) = obj.get("display_name").and_then(Value::as_str)
    {
        let slug = display
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect::<String>();
        if !slug.is_empty() {
            obj.insert("id".to_string(), Value::String(slug));
        }
    }
    if !obj.contains_key("display_name")
        && let Some(id) = obj.get("id").and_then(Value::as_str)
    {
        obj.insert(
            "display_name".to_string(),
            Value::String(title_from_step_name(id)),
        );
    }
    if !obj.contains_key("description") {
        obj.insert(
            "description".to_string(),
            Value::String("Born from Rookery interview.".into()),
        );
    }

    if let Some(Value::Array(penguins)) = obj.remove("penguins") {
        let coerced: Vec<Value> = penguins
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                if let Value::Object(mut po) = p {
                    coerce_penguin_object(&mut po, i);
                    Value::Object(po)
                } else {
                    p
                }
            })
            .collect();
        obj.insert("penguins".to_string(), Value::Array(coerced));
    }

    Value::Object(obj.clone())
}

/// Parse a [`RookeryDraft`] from assistant text (fenced TOML/JSON or raw object).
pub fn parse_draft_from_assistant(text: &str) -> Result<RookeryDraft, KowalskiError> {
    let (body, format) = extract_draft_block(text).ok_or_else(|| {
        KowalskiError::Validation(
            "no draft block found in assistant output (use ```toml or ```json)".into(),
        )
    })?;
    let mut draft = match format {
        DraftBlockFormat::Toml => parse_lenient_toml(&body),
        DraftBlockFormat::Json => {
            let value: Value = serde_json::from_str(&body).map_err(|e| {
                KowalskiError::Validation(format!("invalid RookeryDraft JSON: {e}"))
            })?;
            parse_lenient_json_value(value)
        }
    }?;
    crate::rookery::normalize::normalize_draft(&mut draft);
    crate::rookery::validate::validate_draft(&draft)?;
    Ok(draft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coerces_description_object() {
        let text = r#"```json
{
  "id": "demo",
  "display_name": "Demo",
  "description": { "summary": "A demo horde" },
  "pipeline": ["ingest"],
  "penguins": [{
    "name": "ingest",
    "kind": "ingest",
    "display_name": "Ingest",
    "description": "Collect",
    "prompt_body": "Do ingest",
    "output": { "path": "debug/raw/" }
  }]
}
```"#;
        let d = parse_draft_from_assistant(text).unwrap();
        assert_eq!(d.description, "A demo horde");
        assert_eq!(d.penguins[0].output, "debug/raw/");
    }

    #[test]
    fn coerces_pipeline_objects() {
        let text = r#"{"id":"x","display_name":"X","description":"d","pipeline":[{"name":"collect"},{"name":"deliver"}],"penguins":[{"name":"collect","kind":"ingest","display_name":"C","description":"d","prompt_body":"p","output":"a.md"},{"name":"deliver","kind":"deliver","display_name":"D","description":"d","prompt_body":"p","output":"HANDOFF.md"}]}"#;
        let d = parse_draft_from_assistant(text).unwrap();
        assert_eq!(d.pipeline, vec!["collect", "deliver"]);
    }

    #[test]
    fn parses_fenced_json() {
        let text = r#"Here is the draft:

```json
{"id":"x","display_name":"X","description":"d","pipeline":["a"],"penguins":[{"name":"a","kind":"process","display_name":"A","description":"d","prompt_body":"p","output":"out.md"}]}
```
"#;
        let d = parse_draft_from_assistant(text).unwrap();
        assert_eq!(d.id, "x");
    }

    #[test]
    fn infers_missing_penguin_kind_from_name() {
        let text = r#"```json
{
  "id": "demo",
  "display_name": "Demo",
  "description": "A demo",
  "pipeline": ["ingest", "deliver"],
  "penguins": [
    { "name": "ingest", "display_name": "Ingest", "description": "Collect", "prompt_body": "go", "output": "debug/raw/" },
    { "name": "deliver", "display_name": "Deliver", "description": "Ship", "prompt_body": "go", "output": "HANDOFF.md" }
  ]
}
```"#;
        let d = parse_draft_from_assistant(text).unwrap();
        assert_eq!(d.penguins[0].kind, "ingest");
        assert_eq!(d.penguins[1].kind, "deliver");
    }

    #[test]
    fn coerces_edges_from_json() {
        let text = r#"```json
{
  "id": "fork-demo",
  "display_name": "Fork",
  "description": "fork join",
  "pipeline": ["ingest", "branch-a", "branch-b", "join"],
  "edges": [
    { "from": "ingest", "to": "branch-a" },
    { "from": "ingest", "to": "branch-b" },
    { "from": "branch-a", "to": "join" },
    { "from": "branch-b", "to": "join" }
  ],
  "penguins": [
    { "name": "ingest", "kind": "ingest", "display_name": "Ingest", "description": "d", "prompt_body": "p", "output": "debug/raw/" },
    { "name": "branch-a", "kind": "process", "display_name": "A", "description": "d", "prompt_body": "p", "output": "debug/a.md" },
    { "name": "branch-b", "kind": "process", "display_name": "B", "description": "d", "prompt_body": "p", "output": "debug/b.md" },
    { "name": "join", "kind": "process", "display_name": "Join", "description": "d", "prompt_body": "p", "output": "debug/join.md" }
  ]
}
```"#;
        let draft = parse_draft_from_assistant(text).expect("edges coercion");
        assert_eq!(draft.edges.len(), 4);
        assert!(draft.penguins.iter().find(|p| p.name == "join").unwrap().context_paths.iter().any(|c| c.contains("branch-a")));
    }

    #[test]
    fn parses_minimal_toml_draft() {
        let text = r#"```toml
id = "demo-horde"
display_name = "Demo Horde"
description = "From TOML"

pipeline = ["ingest"]

[[penguins]]
name = "ingest"
description = "Collect sources"
prompt_body = "Ingest operator input."
output = "debug/raw/"
```"#;
        let d = parse_draft_from_assistant(text).unwrap();
        assert_eq!(d.id, "demo-horde");
        assert_eq!(d.penguins[0].kind, "ingest");
        assert_eq!(d.penguins[0].display_name, "Ingest");
    }
}
