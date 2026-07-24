//! In-process filesystem tool (`fs_tool`) for horde coding stages and REPL demos.

use crate::error::KowalskiError;
use crate::tools::internal::file_system::{
    self, list_dir_entries, read_file_bounded, write_file, DEFAULT_MAX_READ_BYTES,
};
use crate::tools::policy::ToolExecutionPolicy;
use crate::tools::{ParameterType, Tool, ToolInput, ToolOutput, ToolParameter};
use async_trait::async_trait;
use std::path::Path;

/// Built-in filesystem tool; callers pass [`ToolExecutionPolicy::sandbox_root`] at execution time
/// via the `sandbox_root` field on [`ToolInput::parameters`] (set by the agent HTTP layer).
#[derive(Debug, Clone, Default)]
pub struct FsTool;

const SANDBOX_KEY: &str = "sandbox_root";

impl FsTool {
    fn policy_from_input(input: &ToolInput) -> ToolExecutionPolicy {
        let sandbox_root = input
            .parameters
            .get(SANDBOX_KEY)
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| Path::new(s).to_path_buf());
        ToolExecutionPolicy {
            allowed_tools: None,
            sandbox_root,
            quiet: false,
        }
    }

    fn resolve_path(input: &ToolInput, key: &str) -> Result<std::path::PathBuf, KowalskiError> {
        let policy = Self::policy_from_input(input);
        let raw = input
            .parameters
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| KowalskiError::ToolInvalidInput(format!("missing `{key}`")))?;
        policy.resolve_sandbox_path(raw)
    }
}

#[async_trait]
impl Tool for FsTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let policy = Self::policy_from_input(&input);
        policy.validate_parameters_paths(&input.parameters)?;

        let task = input.task_type.as_str();
        let result = match task {
            "list_dir" => {
                let path = Self::resolve_path(&input, "path")?;
                let max = input
                    .parameters
                    .get("max_entries")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200) as usize;
                let entries = list_dir_entries(&path, max).map_err(|e| {
                    KowalskiError::ToolExecution(format!("list_dir failed: {e}"))
                })?;
                serde_json::json!({
                    "path": path.display().to_string(),
                    "entries": entries.iter().map(|(name, is_dir)| {
                        serde_json::json!({ "name": name, "is_dir": is_dir })
                    }).collect::<Vec<_>>(),
                })
            }
            "read_file" | "get_file" => {
                let path = Self::resolve_path(&input, "path")?;
                let max = input
                    .parameters
                    .get("max_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(DEFAULT_MAX_READ_BYTES as u64) as usize;
                let body = read_file_bounded(&path, max).map_err(|e| {
                    KowalskiError::ToolExecution(format!("read_file failed: {e}"))
                })?;
                serde_json::json!({
                    "path": path.display().to_string(),
                    "content": body,
                })
            }
            "get_file_first_lines" => {
                let path = Self::resolve_path(&input, "path")?;
                let n = input
                    .parameters
                    .get("num_lines")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;
                let body = read_file_bounded(&path, DEFAULT_MAX_READ_BYTES).map_err(|e| {
                    KowalskiError::ToolExecution(format!("read failed: {e}"))
                })?;
                let lines: Vec<&str> = body.lines().take(n).collect();
                serde_json::json!({
                    "path": path.display().to_string(),
                    "lines": lines,
                })
            }
            "write_file" => {
                let path = Self::resolve_path(&input, "path")?;
                let content = input
                    .parameters
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                write_file(&path, content).map_err(|e| {
                    KowalskiError::ToolExecution(format!("write_file failed: {e}"))
                })?;
                serde_json::json!({
                    "path": path.display().to_string(),
                    "bytes_written": content.len(),
                })
            }
            "mkdir" => {
                let path = Self::resolve_path(&input, "path")?;
                file_system::mkdir_all(&path).map_err(|e| {
                    KowalskiError::ToolExecution(format!("mkdir failed: {e}"))
                })?;
                serde_json::json!({ "path": path.display().to_string() })
            }
            other => {
                return Err(KowalskiError::ToolExecution(format!(
                    "unknown fs_tool task `{other}` (supported: list_dir, read_file, write_file, mkdir, get_file_first_lines)"
                )));
            }
        };
        Ok(ToolOutput::new(result, None))
    }

    fn name(&self) -> &str {
        "fs_tool"
    }

    fn description(&self) -> &str {
        "Sandboxed filesystem operations: list_dir, read_file, write_file, mkdir, get_file_first_lines. \
         Paths must stay under the run project root when a sandbox is active."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "task".into(),
                description: "Operation: list_dir | read_file | write_file | mkdir | get_file_first_lines"
                    .into(),
                required: true,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "path".into(),
                description: "File or directory path (absolute or relative to project root)".into(),
                required: false,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "content".into(),
                description: "File contents for write_file".into(),
                required: false,
                default_value: None,
                parameter_type: ParameterType::String,
            },
            ToolParameter {
                name: "num_lines".into(),
                description: "Line count for get_file_first_lines".into(),
                required: false,
                default_value: Some("10".into()),
                parameter_type: ParameterType::Number,
            },
        ]
    }
}
