use crate::tools::ToolCall;
use llm_json::repair_json;
use once_cell::sync::Lazy;
use regex::Regex;

static CODE_FENCE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)```(?:json)?\s*\n?(.*?)```").expect("CODE_FENCE regex")
});

/// Strips the first ``` or ```json … ``` fenced block if present; otherwise returns `s` unchanged.
pub fn strip_markdown_code_fences(s: &str) -> String {
    if let Some(caps) = CODE_FENCE.captures(s) {
        if let Some(m) = caps.get(1) {
            return m.as_str().trim().to_string();
        }
    }
    s.to_string()
}

/// True if the model output still looks like it tried to emit a tool JSON object but we could not parse any [`ToolCall`].
/// Used to send one self-correction hint in the ReAct loop (WP4).
pub fn looks_like_tool_json_attempt(s: &str) -> bool {
    if !extract_tool_calls(s).is_empty() {
        return false;
    }
    let trimmed = s.trim();
    if trimmed.contains("```") {
        return true;
    }
    trimmed.contains('{')
        && (trimmed.contains("\"name\"") || trimmed.contains("'name'"))
}

fn extract_tool_calls_inner(input: &str) -> Vec<ToolCall> {
    let mut results = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i;
            let mut brace_count = 0;
            let mut in_string = false;
            let mut escaped = false;
            let mut j = i;

            while j < chars.len() {
                let c = chars[j];
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = !in_string;
                } else if !in_string {
                    if c == '{' {
                        brace_count += 1;
                    } else if c == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            let raw_obj: String = chars[start..=j].iter().collect();

                            // Try to repair and parse as ToolCall
                            if let Ok(repaired) =
                                repair_json(&raw_obj, &llm_json::RepairOptions::default())
                            {
                                if let Ok(tool_call) = serde_json::from_str::<ToolCall>(&repaired) {
                                    results.push(tool_call);
                                }
                            }

                            i = j; // Move past this object
                            break;
                        }
                    }
                }
                j += 1;
            }

            // If we reached the end but have unclosed braces, try to repair the whole remaining chunk
            if j == chars.len() && brace_count > 0 {
                let raw_obj: String = chars[start..j].iter().collect();
                if let Ok(repaired) = repair_json(&raw_obj, &llm_json::RepairOptions::default()) {
                    if let Ok(tool_call) = serde_json::from_str::<ToolCall>(&repaired) {
                        results.push(tool_call);
                    }
                }
            }
        }
        i += 1;
    }
    results
}

/// Extracts potential tool calls from a string, repairing malformed JSON if necessary.
/// Strips a leading markdown ```json … ``` fence when present, then runs extraction.
pub fn extract_tool_calls(input: &str) -> Vec<ToolCall> {
    let mut results = extract_tool_calls_inner(input);
    if results.is_empty() {
        let stripped = strip_markdown_code_fences(input);
        if stripped != input {
            results = extract_tool_calls_inner(&stripped);
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tool_call() {
        let input = "Here is a call: {\"name\": \"fs_tool\", \"parameters\": {\"task\": \"list_dir\", \"path\": \"/\"}}";
        let calls = extract_tool_calls(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fs_tool");
    }

    #[test]
    fn test_extract_repaired_call() {
        let input = "Broken: {\"name\": \"fs_tool\", \"parameters\": {\"task\": \"list_dir\"";
        let calls = extract_tool_calls(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fs_tool");

        let input2 = "Messy: {name: \"fs_tool\", parameters: {task: \"list_dir\", path: \"/tmp\"}}";
        let calls2 = extract_tool_calls(input2);
        assert_eq!(calls2.len(), 1);
        assert_eq!(calls2[0].name, "fs_tool");
    }

    #[test]
    fn test_extract_from_markdown_fence() {
        let input = r#"Thought: use tool
```json
{"name": "fs_tool", "parameters": {"task": "list_dir", "path": "/"}}
```"#;
        let calls = extract_tool_calls(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fs_tool");
    }

    #[test]
    fn looks_like_attempt_when_fenced_but_unparseable() {
        let s = r#"```json
{ "name": "oops"
```"#;
        assert!(looks_like_tool_json_attempt(s));
    }

    #[test]
    fn looks_like_attempt_false_when_tool_call_parses() {
        let s = r#"{"name": "fs_tool", "parameters": {}}"#;
        assert!(!looks_like_tool_json_attempt(s));
    }

    #[test]
    fn looks_like_attempt_true_when_tool_shape_but_invalid_json() {
        // Valid-looking object that does not deserialize to [`ToolCall`] (name must be a string).
        let s = r#"{"name": 999, "parameters": {}}"#;
        assert!(
            extract_tool_calls(s).is_empty(),
            "precondition: should not parse as ToolCall"
        );
        assert!(looks_like_tool_json_attempt(s));
    }

    #[test]
    fn looks_like_attempt_false_on_plain_text() {
        assert!(!looks_like_tool_json_attempt("Hello, no JSON here."));
    }
}
