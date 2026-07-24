//! Per-request tool execution constraints (horde workers, sandboxed coding stages).

use crate::error::KowalskiError;
use crate::tools::internal::file_system::try_canonicalize;
use std::path::{Component, Path, PathBuf};

/// Optional allowlist + filesystem sandbox for one chat / horde stage turn.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolExecutionPolicy {
    pub allowed_tools: Option<Vec<String>>,
    pub sandbox_root: Option<PathBuf>,
    /// When true, suppress `[agent]` / tool-loop stdout (horde workers over HTTP).
    pub quiet: bool,
}

impl ToolExecutionPolicy {
    pub fn allows(&self, tool_name: &str) -> bool {
        match &self.allowed_tools {
            Some(list) if !list.is_empty() => list.iter().any(|t| t == tool_name),
            _ => true,
        }
    }

    pub fn ensure_tool_allowed(&self, tool_name: &str) -> Result<(), KowalskiError> {
        if self.allows(tool_name) {
            Ok(())
        } else {
            Err(KowalskiError::ToolExecution(format!(
                "tool `{tool_name}` is not allowed for this stage (allowed: {})",
                self.allowed_tools
                    .as_ref()
                    .map(|v| v.join(", "))
                    .unwrap_or_else(|| "*".into())
            )))
        }
    }

    /// Resolve `path` under optional `sandbox_root`; reject path traversal escapes.
    pub fn resolve_sandbox_path(&self, path: &str) -> Result<PathBuf, KowalskiError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(KowalskiError::ToolInvalidInput(
                "path must not be empty".into(),
            ));
        }
        let candidate = Path::new(trimmed);
        let resolved = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else if let Some(root) = &self.sandbox_root {
            root.join(candidate)
        } else {
            candidate.to_path_buf()
        };
        if let Some(root) = &self.sandbox_root {
            let root_canon = try_canonicalize(root);
            let resolved_canon = try_canonicalize(&resolved);
            if !resolved_canon.starts_with(&root_canon) {
                return Err(KowalskiError::ToolExecution(format!(
                    "path `{}` escapes sandbox root `{}`",
                    trimmed,
                    root_canon.display()
                )));
            }
            Ok(resolved_canon)
        } else {
            Ok(resolved)
        }
    }

    /// Collect path-like parameter values from a tool JSON payload.
    pub fn paths_in_parameters(params: &serde_json::Value) -> Vec<String> {
        const KEYS: &[&str] = &["path", "from", "to", "dst", "src", "source", "target"];
        let mut out = Vec::new();
        if let Some(obj) = params.as_object() {
            for key in KEYS {
                if let Some(v) = obj.get(*key).and_then(|x| x.as_str())
                    && !v.trim().is_empty()
                {
                    out.push(v.to_string());
                }
            }
        }
        out
    }

    pub fn validate_parameters_paths(&self, params: &serde_json::Value) -> Result<(), KowalskiError> {
        if self.sandbox_root.is_none() {
            return Ok(());
        }
        for p in Self::paths_in_parameters(params) {
            for component in Path::new(&p).components() {
                if matches!(component, Component::ParentDir) {
                    return Err(KowalskiError::ToolExecution(format!(
                        "path `{p}` must not contain `..`"
                    )));
                }
            }
            let _ = self.resolve_sandbox_path(&p)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        let policy = ToolExecutionPolicy {
            allowed_tools: None,
            sandbox_root: Some(dir.path().to_path_buf()),
            quiet: false,
        };
        assert!(policy.resolve_sandbox_path("/etc/passwd").is_err());
        assert!(policy
            .resolve_sandbox_path(&format!("{}/../", dir.path().display()))
            .is_err());
    }

    #[test]
    fn allowlist_blocks_unknown_tool() {
        let policy = ToolExecutionPolicy {
            allowed_tools: Some(vec!["fs_tool".into()]),
            sandbox_root: None,
            quiet: false,
        };
        assert!(policy.ensure_tool_allowed("fs_tool").is_ok());
        assert!(policy.ensure_tool_allowed("other").is_err());
    }
}
