//! Non-LLM horde stage helpers: **verify** (run shell command) and **apply** (patch dry-run / gated execute).

use crate::error::KowalskiError;
use crate::tools::internal::file_system::try_canonicalize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Default cap on combined stdout+stderr captured from verify commands.
pub const DEFAULT_VERIFY_MAX_OUTPUT_BYTES: usize = 512 * 1024;

/// Default wall-clock limit for verify subprocess (best-effort via wait loop).
pub const DEFAULT_VERIFY_TIMEOUT_SECS: u64 = 600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyRunResult {
    pub command: String,
    pub cwd: PathBuf,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub timed_out: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Pass,
    Fail,
}

impl StageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pass" | "ok" | "success" => Some(Self::Pass),
            "fail" | "failed" | "error" => Some(Self::Fail),
            _ => None,
        }
    }
}

/// Resolve working directory: `project_path` + optional relative `verify_cwd`.
pub fn resolve_verify_cwd(project_path: &Path, verify_cwd: Option<&str>) -> Result<PathBuf, KowalskiError> {
    if !project_path.is_dir() {
        return Err(KowalskiError::Validation(format!(
            "verify cwd: project path is not a directory: {}",
            project_path.display()
        )));
    }
    let root = try_canonicalize(project_path);
    let cwd = match verify_cwd.map(str::trim).filter(|s| !s.is_empty()) {
        Some(rel) => {
            let p = Path::new(rel);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                root.join(rel)
            }
        }
        None => root,
    };
    if !cwd.is_dir() {
        return Err(KowalskiError::Validation(format!(
            "verify cwd: `{}` is not a directory",
            cwd.display()
        )));
    }
    Ok(try_canonicalize(&cwd))
}

fn clip_output(text: &str, max_bytes: usize, truncated: &mut bool) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    *truncated = true;
    String::from_utf8_lossy(&text.as_bytes()[..max_bytes]).into_owned()
}

fn run_shell_command(command: &str, cwd: &Path) -> Result<Output, String> {
    #[cfg(unix)]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.arg("-c").arg(command);
        c
    };
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    };
    cmd.current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.output().map_err(|e| format!("spawn failed: {e}"))
}

/// Run `command` in `cwd`, capturing stdout/stderr (truncated at `max_output_bytes` each).
pub fn run_verify_command(
    command: &str,
    cwd: &Path,
    max_output_bytes: usize,
    timeout: Duration,
) -> VerifyRunResult {
    let started = Instant::now();
    let command = command.trim();
    let mut timed_out = false;
    let mut truncated = false;

    let output = match run_shell_command(command, cwd) {
        Ok(o) => o,
        Err(e) => {
            return VerifyRunResult {
                command: command.to_string(),
                cwd: cwd.to_path_buf(),
                exit_code: None,
                success: false,
                stdout: String::new(),
                stderr: e,
                duration_ms: started.elapsed().as_millis() as u64,
                timed_out: false,
                truncated: false,
            };
        }
    };

    if started.elapsed() > timeout {
        timed_out = true;
    }

    let exit_code = output.status.code();
    let success = output.status.success() && !timed_out;
    let stdout = clip_output(
        &String::from_utf8_lossy(&output.stdout),
        max_output_bytes,
        &mut truncated,
    );
    let stderr = clip_output(
        &String::from_utf8_lossy(&output.stderr),
        max_output_bytes,
        &mut truncated,
    );

    VerifyRunResult {
        command: command.to_string(),
        cwd: cwd.to_path_buf(),
        exit_code,
        success,
        stdout,
        stderr,
        duration_ms: started.elapsed().as_millis() as u64,
        timed_out,
        truncated,
    }
}

pub fn verify_status(result: &VerifyRunResult) -> StageStatus {
    if result.success {
        StageStatus::Pass
    } else {
        StageStatus::Fail
    }
}

/// Markdown artifact with YAML frontmatter (`status`, `exit_code`, …) for downstream routing (CA-4).
pub fn format_verify_artifact(result: &VerifyRunResult) -> String {
    let status = verify_status(result);
    let exit_display = result
        .exit_code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "none".into());
    let mut doc = format!(
        "---\nstatus: {}\nexit_code: {}\ncommand: {}\ncwd: {}\nduration_ms: {}\ntimed_out: {}\ntruncated: {}\n---\n\n",
        status.as_str(),
        exit_display,
        result.command.replace('\n', " "),
        result.cwd.display(),
        result.duration_ms,
        result.timed_out,
        result.truncated,
    );
    doc.push_str("# Verify\n\n");
    doc.push_str(&format!("**Status:** {} (exit `{}`)\n\n", status.as_str(), exit_display));
    if result.timed_out {
        doc.push_str("_Note: command exceeded configured timeout._\n\n");
    }
    if result.truncated {
        doc.push_str("_Note: output truncated to capture limit._\n\n");
    }
    doc.push_str("## stdout\n\n```text\n");
    doc.push_str(&result.stdout);
    if !result.stdout.ends_with('\n') {
        doc.push('\n');
    }
    doc.push_str("```\n\n## stderr\n\n```text\n");
    doc.push_str(&result.stderr);
    if !result.stderr.ends_with('\n') {
        doc.push('\n');
    }
    doc.push_str("```\n");
    doc
}

/// Short operator-facing tail from a verify artifact (stderr preferred).
pub fn verify_output_excerpt(content: &str, max_chars: usize) -> String {
    let section = content
        .find("## stderr")
        .map(|i| &content[i..])
        .or_else(|| content.find("## stdout").map(|i| &content[i..]))
        .unwrap_or(content);
    section.chars().take(max_chars).collect()
}

/// Read `status:` from YAML frontmatter in a verify artifact.
pub fn parse_stage_status_from_artifact(content: &str) -> Option<StageStatus> {
    if !content.starts_with("---") {
        return None;
    }
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let fm = &rest[..end];
    for line in fm.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if k.trim() == "status" {
            return StageStatus::parse(v);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyDryRunResult {
    pub patch_blocks: usize,
    pub checked: bool,
    pub apply_output: String,
    pub success: bool,
}

/// Extract unified diff fenced blocks from a markdown artifact.
pub fn extract_unified_diffs(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_diff = false;
    let mut buf = String::new();
    for line in content.lines() {
        if line.starts_with("```diff") || line.starts_with("```patch") {
            in_diff = true;
            buf.clear();
            continue;
        }
        if in_diff && line.starts_with("```") {
            in_diff = false;
            let trimmed = buf.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            continue;
        }
        if in_diff {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    out
}

/// Dry-run: write diffs to temp files and run `patch --dry-run` when available.
pub fn apply_patches_dry_run(project_path: &Path, previous_artifact: &str) -> ApplyDryRunResult {
    let diffs = extract_unified_diffs(previous_artifact);
    if diffs.is_empty() {
        return ApplyDryRunResult {
            patch_blocks: 0,
            checked: false,
            apply_output: "No ```diff blocks found in upstream artifact; nothing to apply (dev stages may have used fs_tool writes directly).".into(),
            success: true,
        };
    }
    let tmp_dir = std::env::temp_dir().join(format!(
        "kowalski-apply-{}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&tmp_dir);
    let mut notes = Vec::new();
    let mut all_ok = true;
    for (idx, diff) in diffs.iter().enumerate() {
        let patch_path = tmp_dir.join(format!("patch-{idx}.diff"));
        if fs::write(&patch_path, diff).is_err() {
            all_ok = false;
            notes.push(format!("patch-{idx}: failed to write temp file"));
            continue;
        }
        #[cfg(unix)]
        let check = Command::new("patch")
            .arg("-p1")
            .arg("--dry-run")
            .arg("-i")
            .arg(&patch_path)
            .current_dir(project_path)
            .output();
        #[cfg(not(unix))]
        let check: Result<Output, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "patch dry-run is unix-only in this MVP",
        ));
        match check {
            Ok(o) if o.status.success() => {
                notes.push(format!("patch-{idx}: dry-run OK"));
            }
            Ok(o) => {
                all_ok = false;
                notes.push(format!(
                    "patch-{idx}: dry-run failed (exit {:?}): {}",
                    o.status.code(),
                    String::from_utf8_lossy(&o.stderr)
                ));
            }
            Err(e) => {
                all_ok = false;
                notes.push(format!("patch-{idx}: {e}"));
            }
        }
    }
    ApplyDryRunResult {
        patch_blocks: diffs.len(),
        checked: true,
        apply_output: notes.join("\n"),
        success: all_ok,
    }
}

pub fn format_apply_artifact(mode: &str, result: &ApplyDryRunResult, executed: bool) -> String {
    let status = if result.success {
        StageStatus::Pass
    } else {
        StageStatus::Fail
    };
    format!(
        "---\nstatus: {}\nmode: {}\nexecuted: {}\npatch_blocks: {}\n---\n\n# Apply\n\n- Mode: `{mode}`\n- Executed: {executed}\n- Patch blocks: {}\n\n## Result\n\n{}\n",
        status.as_str(),
        mode,
        executed,
        result.patch_blocks,
        result.patch_blocks,
        result.apply_output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_echo_ok() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_verify_command(
            "echo verify-ok",
            dir.path(),
            4096,
            Duration::from_secs(30),
        );
        assert!(result.success, "{:?}", result);
        assert!(result.stdout.contains("verify-ok"));
    }

    #[test]
    fn verify_artifact_frontmatter_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_verify_command("false", dir.path(), 4096, Duration::from_secs(30));
        let md = format_verify_artifact(&result);
        assert_eq!(
            parse_stage_status_from_artifact(&md),
            Some(StageStatus::Fail)
        );
    }

    #[test]
    fn extract_diff_blocks() {
        let md = "Plan\n\n```diff\n--- a/foo\n+++ b/foo\n@@\n+x\n```\n";
        assert_eq!(extract_unified_diffs(md).len(), 1);
    }
}
