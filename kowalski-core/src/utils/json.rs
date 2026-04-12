use crate::tools::ToolCall;
use llm_json::repair_json;

/// Extracts potential tool calls from a string, repairing malformed JSON if necessary.
pub fn extract_tool_calls(input: &str) -> Vec<ToolCall> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_tool_call() {
        let input = "Here is a call: {\"name\": \"fs_tool\", \"parameters\": {\"task\": \"list_dir\", \"path\": \"/\"}}";
        let calls = extract_tool_calls(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fs_tool");
    }

    #[test]
    fn test_extract_repaired_call() {
        // Missing closing quotes/braces etc. that llm_json can fix
        let input = "Broken: {\"name\": \"fs_tool\", \"parameters\": {\"task\": \"list_dir\"";
        let calls = extract_tool_calls(input);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fs_tool");

        // Another case: messy content
        let input2 = "Messy: {name: \"fs_tool\", parameters: {task: \"list_dir\", path: \"/tmp\"}}";
        let calls2 = extract_tool_calls(input2);
        assert_eq!(calls2.len(), 1);
        assert_eq!(calls2[0].name, "fs_tool");
    }
}
