//! Parse `RookeryDraft` JSON from builder assistant output.

use crate::error::KowalskiError;
use crate::rookery::types::RookeryDraft;
use serde_json::{Map, Value};

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

/// Extract JSON from a fenced code block or raw `{...}` payload.
pub fn extract_json_block(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed.to_string());
    }
    for marker in ["```json", "```JSON", "```"] {
        if let Some(start) = trimmed.find(marker) {
            let rest = &trimmed[start + marker.len()..];
            let rest = rest.strip_prefix('\n').unwrap_or(rest);
            if let Some(end) = rest.find("```") {
                let inner = rest[..end].trim();
                if !inner.is_empty() {
                    return Some(inner.to_string());
                }
            }
        }
    }
    None
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

fn coerce_penguin_object(obj: &mut Map<String, Value>) {
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
    if !obj.contains_key("tool_ids") {
        if let Some(v) = obj.remove("tools") {
            obj.insert("tool_ids".to_string(), v);
        }
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

    if let Some(Value::Array(penguins)) = obj.remove("penguins") {
        let coerced: Vec<Value> = penguins
            .into_iter()
            .map(|p| {
                if let Value::Object(mut po) = p {
                    coerce_penguin_object(&mut po);
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

/// Parse a [`RookeryDraft`] from assistant text (fenced JSON or raw object).
pub fn parse_draft_from_assistant(text: &str) -> Result<RookeryDraft, KowalskiError> {
    let json_str = extract_json_block(text)
        .ok_or_else(|| KowalskiError::Validation("no JSON draft found in assistant output".into()))?;
    let value: Value = serde_json::from_str(&json_str).map_err(|e| {
        KowalskiError::Validation(format!("invalid RookeryDraft JSON: {e}"))
    })?;
    let coerced = coerce_draft_value(value);
    serde_json::from_value::<RookeryDraft>(coerced).map_err(|e| {
        KowalskiError::Validation(format!("invalid RookeryDraft JSON: {e}"))
    })
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
}
