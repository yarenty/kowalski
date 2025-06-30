use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use kowalski_core::error::KowalskiError;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput, ToolParameter};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// List files and directories in the given path.
pub fn list_dir<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>, FsError> {
    let entries = fs::read_dir(&path)?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry?;
        result.push(entry.path());
    }
    Ok(result)
}

/// Recursively find files matching a pattern in a directory.
pub fn find_files<P: AsRef<Path>>(dir: P, pattern: &str) -> Result<Vec<PathBuf>, FsError> {
    let mut result = Vec::new();
    find_files_recursive(dir.as_ref(), pattern, &mut result)?;
    Ok(result)
}

fn find_files_recursive(dir: &Path, pattern: &str, result: &mut Vec<PathBuf>) -> Result<(), FsError> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_files_recursive(&path, pattern, result)?;
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains(pattern) {
                    result.push(path);
                }
            }
        }
    }
    Ok(())
}

/// Get the full contents of a file as a String.
pub fn get_file_contents<P: AsRef<Path>>(path: P) -> Result<String, FsError> {
    let contents = fs::read_to_string(&path)?;
    Ok(contents)
}

/// Get the first `num_lines` lines of a file as a Vec<String>.
pub fn get_file_first_lines<P: AsRef<Path>>(path: P, num_lines: usize) -> Result<Vec<String>, FsError> {
    let file = fs::File::open(&path)?;
    let reader = io::BufReader::new(file);
    Ok(reader.lines().take(num_lines).collect::<Result<_, _>>()?)
}

/// Get the last `num_lines` lines of a file as a Vec<String>.
pub fn get_file_last_lines<P: AsRef<Path>>(path: P, num_lines: usize) -> Result<Vec<String>, FsError> {
    let file = fs::File::open(&path)?;
    let reader = io::BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        lines.push(line?);
    }
    let len = lines.len();
    let start = if num_lines > len { 0 } else { len - num_lines };
    Ok(lines[start..].to_vec())
}

pub struct FsTool;

impl FsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FsTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        println!("[DEBUG][fs_tool] execute called: task={:?}, parameters={:?}", input.task_type, input.parameters);
        let task = input.parameters.get("task").and_then(|v| v.as_str()).unwrap_or(&input.task_type);
        match task {
            "list_dir" => {
                let path = input.parameters.get("path").and_then(|v| v.as_str()).ok_or_else(||
                    KowalskiError::ToolExecution("Missing 'path' parameter for list_dir".to_string())
                )?;
                let entries = list_dir(path).map_err(|e| KowalskiError::ToolExecution(e.to_string()))?;
                Ok(ToolOutput::new(json!({"entries": entries}), None))
            }
            "find_files" => {
                let dir = input.parameters.get("dir").and_then(|v| v.as_str()).ok_or_else(||
                    KowalskiError::ToolExecution("Missing 'dir' parameter for find_files".to_string())
                )?;
                let pattern = input.parameters.get("pattern").and_then(|v| v.as_str()).ok_or_else(||
                    KowalskiError::ToolExecution("Missing 'pattern' parameter for find_files".to_string())
                )?;
                let files = find_files(dir, pattern).map_err(|e| KowalskiError::ToolExecution(e.to_string()))?;
                Ok(ToolOutput::new(json!({"files": files}), None))
            }
            "get_file_contents" => {
                let path = input.parameters.get("path").and_then(|v| v.as_str()).ok_or_else(||
                    KowalskiError::ToolExecution("Missing 'path' parameter for get_file_contents".to_string())
                )?;
                let contents = get_file_contents(path).map_err(|e| KowalskiError::ToolExecution(e.to_string()))?;
                Ok(ToolOutput::new(json!({"contents": contents}), None))
            }
            "get_file_first_lines" => {
                let path = input.parameters.get("path").and_then(|v| v.as_str()).ok_or_else(||
                    KowalskiError::ToolExecution("Missing 'path' parameter for get_file_first_lines".to_string())
                )?;
                let num_lines = input.parameters.get("num_lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let lines = get_file_first_lines(path, num_lines).map_err(|e| KowalskiError::ToolExecution(e.to_string()))?;
                Ok(ToolOutput::new(json!({"lines": lines}), None))
            }
            "get_file_last_lines" => {
                let path = input.parameters.get("path").and_then(|v| v.as_str()).ok_or_else(||
                    KowalskiError::ToolExecution("Missing 'path' parameter for get_file_last_lines".to_string())
                )?;
                let num_lines = input.parameters.get("num_lines").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let lines = get_file_last_lines(path, num_lines).map_err(|e| KowalskiError::ToolExecution(e.to_string()))?;
                Ok(ToolOutput::new(json!({"lines": lines}), None))
            }
            _ => Err(KowalskiError::ToolExecution(format!("Unsupported task type: {}", task))),
        }
    }

    fn name(&self) -> &str {
        "fs_tool"
    }

    fn description(&self) -> &str {
        "Filesystem tool for directory listing, file search, and file content retrieval."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter {
                name: "task".to_string(),
                description: "The filesystem operation to perform (list_dir, find_files, get_file_contents, get_file_first_lines, get_file_last_lines)".to_string(),
                required: true,
                default_value: None,
                parameter_type: kowalski_core::tools::ParameterType::String,
            },
            ToolParameter {
                name: "path".to_string(),
                description: "Path to the file or directory (for list_dir, get_file_contents, get_file_first_lines, get_file_last_lines)".to_string(),
                required: false,
                default_value: None,
                parameter_type: kowalski_core::tools::ParameterType::String,
            },
            ToolParameter {
                name: "dir".to_string(),
                description: "Directory to search in (for find_files)".to_string(),
                required: false,
                default_value: None,
                parameter_type: kowalski_core::tools::ParameterType::String,
            },
            ToolParameter {
                name: "pattern".to_string(),
                description: "Pattern to match files (for find_files)".to_string(),
                required: false,
                default_value: None,
                parameter_type: kowalski_core::tools::ParameterType::String,
            },
            ToolParameter {
                name: "num_lines".to_string(),
                description: "Number of lines to read (for get_file_first_lines, get_file_last_lines)".to_string(),
                required: false,
                default_value: Some("10".to_string()),
                parameter_type: kowalski_core::tools::ParameterType::Number,
            },
        ]
    }
} 