//! Parse `RookeryDraft` JSON from builder assistant output.

use crate::error::KowalskiError;
use crate::rookery::types::RookeryDraft;

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

/// Parse a [`RookeryDraft`] from assistant text (fenced JSON or raw object).
pub fn parse_draft_from_assistant(text: &str) -> Result<RookeryDraft, KowalskiError> {
    let json_str = extract_json_block(text)
        .ok_or_else(|| KowalskiError::Validation("no JSON draft found in assistant output".into()))?;
    serde_json::from_str::<RookeryDraft>(&json_str).map_err(|e| {
        KowalskiError::Validation(format!("invalid RookeryDraft JSON: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
