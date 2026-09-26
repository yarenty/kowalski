use crate::tools::ToolCall;
use llm_json::repair_json;
use once_cell::sync::Lazy;
use regex::Regex;

static CODE_FENCE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```(?:json)?\s*\n?(.*?)```").expect("CODE_FENCE regex"));

/// Strips the first ``` or ```json … ``` fenced block if present; otherwise returns `s` unchanged.
pub fn strip_markdown_code_fences(s: &str) -> String {
    if let Some(caps) = CODE_FENCE.captures(s)
        && let Some(m) = caps.get(1)
    {
        return m.as_str().trim().to_string();
    }
    s.to_string()
}

/// True if the model output still looks like it tried to emit a tool JSON object but we could not parse any [`ToolCall`].
/// Used to send one self-correction hint in the ReAct loop.
///
/// An ordinary answer that contains a code block (a shell command, a JSON example) is *not* an
/// attempt: only an object naming a tool with its arguments, or a broken object with a `name`
/// key where a tool call would go, counts.
pub fn looks_like_tool_json_attempt(s: &str) -> bool {
    if !extract_tool_calls(s).is_empty() {
        return false;
    }
    let trimmed = s.trim();
    let has_name = trimmed.contains("\"name\"") || trimmed.contains("'name'");
    if !has_name {
        return false;
    }
    let has_args = ["\"parameters\"", "\"arguments\"", "'parameters'", "'arguments'"]
        .iter()
        .any(|k| trimmed.contains(k));
    if has_args {
        return true;
    }
    // the reply (or its first json/untagged fence) is an object that does not parse
    let body = strip_markdown_code_fences(trimmed);
    let body = body.trim();
    body.starts_with('{') && serde_json::from_str::<serde_json::Value>(body).is_err()
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
                                && let Ok(tool_call) = serde_json::from_str::<ToolCall>(&repaired)
                            {
                                results.push(tool_call);
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
                if let Ok(repaired) = repair_json(&raw_obj, &llm_json::RepairOptions::default())
                    && let Ok(tool_call) = serde_json::from_str::<ToolCall>(&repaired)
                {
                    results.push(tool_call);
                }
            }
        }
        i += 1;
    }
    results
}

/// The first balanced `{…}` object in `input` (string-and-escape aware); when braces never
/// close, the tail from the first `{` (so JSON repair can finish it).
fn first_object_slice(input: &str) -> Option<&str> {
    let bytes = input.as_bytes();
    let start = input.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if escaped {
            escaped = false;
        } else if b == b'\\' {
            escaped = true;
        } else if b == b'"' {
            in_string = !in_string;
        } else if !in_string {
            if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth -= 1;
                if depth == 0 {
                    return Some(&input[start..=i]);
                }
            }
        }
    }
    Some(&input[start..])
}

/// Fallback extractor for models without constrained structured output: recover the first
/// JSON object from a free-form reply (plain JSON, JSON inside prose, ```json fences, or a
/// truncated/sloppy object that JSON repair can fix). Returns `None` when nothing in the
/// input parses to a JSON object.
pub fn extract_first_json_object(input: &str) -> Option<serde_json::Value> {
    let stripped = strip_markdown_code_fences(input);
    for candidate in [stripped.as_str(), input] {
        if let Some(slice) = first_object_slice(candidate)
            && let Ok(repaired) = repair_json(slice, &llm_json::RepairOptions::default())
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&repaired)
            && value.is_object()
        {
            return Some(value);
        }
    }
    None
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
    fn an_answer_with_a_code_block_is_not_a_tool_attempt() {
        let s = "- A rookery is where penguins breed.\n\nTo list files:\n\n```bash\nls -la\n```";
        assert!(!looks_like_tool_json_attempt(s));
        let example = "A person record looks like this:\n\n```json\n{\"name\": \"Ada\", \"age\": 36}\n```";
        assert!(!looks_like_tool_json_attempt(example), "a valid JSON example is an answer");
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

    /// Table of reply shapes an unconstrained model actually produces.
    #[test]
    fn extract_first_json_object_table() {
        let expected = serde_json::json!({"ops": [{"op": "remove_step", "step_id": "a"}]});
        let cases: &[&str] = &[
            // plain JSON, nothing else
            r#"{"ops": [{"op": "remove_step", "step_id": "a"}]}"#,
            // fenced with language tag
            "Here you go:\n```json\n{\"ops\": [{\"op\": \"remove_step\", \"step_id\": \"a\"}]}\n```",
            // fenced without language tag
            "```\n{\"ops\": [{\"op\": \"remove_step\", \"step_id\": \"a\"}]}\n```",
            // bare JSON inside prose
            "Sure! The batch is {\"ops\": [{\"op\": \"remove_step\", \"step_id\": \"a\"}]} — done.",
            // prose before AND after a fence
            "Thinking...\n```json\n{\"ops\": [{\"op\": \"remove_step\", \"step_id\": \"a\"}]}\n```\nLet me know!",
        ];
        for case in cases {
            assert_eq!(
                extract_first_json_object(case).as_ref(),
                Some(&expected),
                "case: {case}"
            );
        }
    }

    #[test]
    fn extract_first_json_object_repairs_sloppy_json() {
        // unquoted keys + trailing truncation
        let truncated = r#"{"ops": [{"op": "remove_step", "step_id": "a""#;
        let v = extract_first_json_object(truncated).expect("repairable");
        assert_eq!(v["ops"][0]["op"], "remove_step");

        let unquoted = "{ops: [{op: \"remove_step\", step_id: \"a\"}]}";
        let v = extract_first_json_object(unquoted).expect("repairable");
        assert_eq!(v["ops"][0]["step_id"], "a");
    }

    #[test]
    fn extract_first_json_object_none_on_plain_text() {
        assert!(extract_first_json_object("No JSON to be found here.").is_none());
        assert!(extract_first_json_object("").is_none());
        // a bare array is not an object
        assert!(extract_first_json_object("[1, 2, 3]").is_none());
    }
}
