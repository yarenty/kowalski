//! In-process step execution for horde runs: [`StepHandler`] trait + [`StepHandlerRegistry`].
//!
//! Deterministic step kinds (`verify`, `apply`, `ingest`) run inside the server process
//! instead of a spawned federation worker: same artifacts, same pass/fail routing on
//! conditional edges, no worker process. LLM kinds stay on the worker path until they
//! move in-process. Adding a step kind is one trait impl plus one registry line:
//!
//! ```ignore
//! struct MyHandler;
//! #[async_trait]
//! impl StepHandler for MyHandler {
//!     fn kind(&self) -> &'static str { "my-kind" }
//!     async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> { … }
//! }
//! registry.register(Arc::new(MyHandler));
//! ```

use crate::error::KowalskiError;
use crate::horde_stages::{
    DEFAULT_VERIFY_MAX_OUTPUT_BYTES, DEFAULT_VERIFY_TIMEOUT_SECS, StageStatus,
    apply_patches_dry_run, format_apply_artifact, format_verify_artifact,
    resolve_verify_cwd, run_verify_command, verify_output_excerpt, verify_status,
};
use crate::llm::provider::LLMProvider;
use crate::source_bundle::{
    extract_project_path_from_source, ingest_assets_markdown, parse_input_assets,
};
use crate::tools::manager::ToolManager;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Debug subtree of a horde workdir where step artifacts collect (worker convention).
pub const WORKDIR_DEBUG_DIR: &str = "debug";

/// Env gate for `apply` steps with `apply_mode = "execute"`.
pub const APPLY_EXECUTE_ENV: &str = "KOWALSKI_HORDE_APPLY";

pub type StepError = KowalskiError;

/// The slice of a sub-agent's spec a step handler needs (captured in the run's
/// manifest snapshot; the orchestrator builds it per delegation).
#[derive(Debug, Clone, Default)]
pub struct StepSpec {
    pub name: String,
    pub kind: String,
    /// Artifact path relative to the horde workdir.
    pub output: Option<String>,
    /// Shell command for `kind = "verify"`.
    pub verify_command: Option<String>,
    /// Working directory relative to the operator `project_path`.
    pub verify_cwd: Option<String>,
    /// `dry-run` (default) or `execute` for `kind = "apply"`.
    pub apply_mode: Option<String>,
    pub tool_ids: Vec<String>,
}

/// Progress sink for handler feed messages (the orchestrator publishes them as
/// `AgentMessage` on the run's federation topic, matching the worker path).
#[async_trait]
pub trait StepEventSink: Send + Sync {
    async fn message(&self, text: &str);
}

/// Sink that drops all messages (tests / detached execution).
pub struct NullEventSink;

#[async_trait]
impl StepEventSink for NullEventSink {
    async fn message(&self, _text: &str) {}
}

/// Everything a handler may need to execute one step of one run.
/// LLM/tool handles are unused by the deterministic kinds and carried for the
/// LLM step kinds when they move in-process.
pub struct StepContext<'a> {
    pub run_id: &'a str,
    pub horde_id: &'a str,
    pub step: &'a StepSpec,
    /// Horde workdir (artifacts root for `StepSpec::output`).
    pub workdir: &'a Path,
    /// Horde definition root (agents/, prompts/).
    pub horde_root: &'a Path,
    /// Operator source input (URLs, file paths, or plain text).
    pub source: Option<&'a str>,
    pub question: &'a str,
    /// Operator project root (resolved from the run form / source block).
    pub project_path: Option<PathBuf>,
    /// Artifact path of the single predecessor step, when any.
    pub previous_artifact: Option<&'a Path>,
    pub events: &'a dyn StepEventSink,
    pub llm: Option<Arc<dyn LLMProvider>>,
    pub tools: Option<Arc<ToolManager>>,
    /// Cooperative cancellation slot (checked between phases; full plumbing
    /// arrives with in-process LLM steps).
    pub cancel: &'a CancellationToken,
}

impl StepContext<'_> {
    /// Operator project root: explicit `project_path`, else parsed out of `source`.
    fn require_project(&self) -> Result<PathBuf, StepError> {
        if let Some(p) = &self.project_path
            && p.is_dir()
        {
            return Ok(p.clone());
        }
        if let Some(s) = self.source
            && let Some(p) = extract_project_path_from_source(s)
        {
            return Ok(p);
        }
        Err(KowalskiError::Validation(
            "verify/apply: missing operator project_path (set on ingest form)".into(),
        ))
    }

    /// Resolve `StepSpec::output` under the workdir, creating parent directories.
    fn artifact_path(&self) -> Result<PathBuf, StepError> {
        let rel = self.step.output.as_deref().ok_or_else(|| {
            KowalskiError::Validation(format!("{} stage `{}` missing `output`", self.step.kind, self.step.name))
        })?;
        let out = self.workdir.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                KowalskiError::Validation(format!("create artifact dir {}: {e}", parent.display()))
            })?;
        }
        Ok(out)
    }

    fn check_cancelled(&self) -> Result<(), StepError> {
        if self.cancel.is_cancelled() {
            return Err(KowalskiError::Validation(format!(
                "step `{}` cancelled",
                self.step.name
            )));
        }
        Ok(())
    }
}

/// Result of one step execution. `status` reuses the [`StageStatus`] vocabulary
/// so conditional edges (`when = "pass" | "fail"`) keep working unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Completed {
        artifact: Option<PathBuf>,
        status: StageStatus,
        summary: String,
    },
}

#[async_trait]
pub trait StepHandler: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError>;
}

/// Kind → handler map, built once at server startup.
#[derive(Clone, Default)]
pub struct StepHandlerRegistry {
    handlers: HashMap<&'static str, Arc<dyn StepHandler>>,
}

impl StepHandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registry with the built-in deterministic kinds: `verify`, `apply`, `ingest`.
    pub fn with_builtin_deterministic() -> Self {
        let mut r = Self::new();
        r.register(Arc::new(VerifyStepHandler));
        r.register(Arc::new(ApplyStepHandler));
        r.register(Arc::new(IngestStepHandler));
        r
    }

    pub fn register(&mut self, handler: Arc<dyn StepHandler>) {
        self.handlers.insert(handler.kind(), handler);
    }

    pub fn get(&self, kind: &str) -> Option<Arc<dyn StepHandler>> {
        self.handlers.get(kind).cloned()
    }

    pub fn contains(&self, kind: &str) -> bool {
        self.handlers.contains_key(kind)
    }

    pub fn kinds(&self) -> Vec<&'static str> {
        let mut kinds: Vec<_> = self.handlers.keys().copied().collect();
        kinds.sort_unstable();
        kinds
    }
}

/// `verify`: run the stage's shell command in the operator project, write the
/// markdown artifact with `status:` frontmatter that drives conditional edges.
pub struct VerifyStepHandler;

#[async_trait]
impl StepHandler for VerifyStepHandler {
    fn kind(&self) -> &'static str {
        "verify"
    }

    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> {
        ctx.check_cancelled()?;
        ctx.events
            .message(&format!("Verify stage `{}` (run command)", ctx.step.name))
            .await;
        let command = ctx
            .step
            .verify_command
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                KowalskiError::Validation(format!(
                    "verify stage `{}` missing `verify_command`",
                    ctx.step.name
                ))
            })?
            .to_string();
        let project = ctx.require_project()?;
        let cwd = resolve_verify_cwd(&project, ctx.step.verify_cwd.as_deref())?;
        let out_path = ctx.artifact_path()?;

        let result = tokio::task::spawn_blocking(move || {
            run_verify_command(
                &command,
                &cwd,
                DEFAULT_VERIFY_MAX_OUTPUT_BYTES,
                Duration::from_secs(DEFAULT_VERIFY_TIMEOUT_SECS),
            )
        })
        .await
        .map_err(|e| KowalskiError::Validation(format!("verify task join: {e}")))?;

        let status = verify_status(&result);
        let doc = format_verify_artifact(&result);
        std::fs::write(&out_path, &doc).map_err(|e| {
            KowalskiError::Validation(format!("write verify artifact {}: {e}", out_path.display()))
        })?;
        let excerpt = verify_output_excerpt(&doc, 1_200);
        if !excerpt.trim().is_empty() {
            ctx.events
                .message(&format!("Verify output ({}):\n\n{excerpt}", status.as_str()))
                .await;
        }
        Ok(StepOutcome::Completed {
            summary: format!("verify `{}`: {}", status.as_str(), out_path.display()),
            artifact: Some(out_path),
            status,
        })
    }
}

/// `apply`: patch dry-run (or env-gated execute) of ```diff blocks from the
/// previous step's artifact against the operator project.
pub struct ApplyStepHandler;

#[async_trait]
impl StepHandler for ApplyStepHandler {
    fn kind(&self) -> &'static str {
        "apply"
    }

    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> {
        ctx.check_cancelled()?;
        ctx.events
            .message(&format!("Apply stage `{}` (patch dry-run)", ctx.step.name))
            .await;
        let prev = ctx
            .previous_artifact
            .ok_or_else(|| KowalskiError::Validation("apply: missing previous_artifact".into()))?;
        let prev_body = std::fs::read_to_string(prev).map_err(|e| {
            KowalskiError::Validation(format!("apply: read {}: {e}", prev.display()))
        })?;
        let project = ctx.require_project()?;
        let mode = ctx
            .step
            .apply_mode
            .clone()
            .unwrap_or_else(|| "dry-run".into());
        let execute = mode.eq_ignore_ascii_case("execute")
            && std::env::var(APPLY_EXECUTE_ENV)
                .ok()
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        let out_path = ctx.artifact_path()?;

        let dry = tokio::task::spawn_blocking(move || apply_patches_dry_run(&project, &prev_body))
            .await
            .map_err(|e| KowalskiError::Validation(format!("apply task join: {e}")))?;

        let status = if dry.success {
            StageStatus::Pass
        } else {
            StageStatus::Fail
        };
        let doc = format_apply_artifact(&mode, &dry, execute);
        std::fs::write(&out_path, doc).map_err(|e| {
            KowalskiError::Validation(format!("write apply artifact {}: {e}", out_path.display()))
        })?;
        Ok(StepOutcome::Completed {
            summary: format!("apply artifact: {}", out_path.display()),
            artifact: Some(out_path),
            status,
        })
    }
}

/// `ingest`: capture operator source tokens (URLs, file/dir paths, text) into
/// the raw-collection markdown under `workdir/debug/` (source_bundle walk).
pub struct IngestStepHandler;

#[async_trait]
impl StepHandler for IngestStepHandler {
    fn kind(&self) -> &'static str {
        "ingest"
    }

    async fn execute(&self, ctx: &StepContext<'_>) -> Result<StepOutcome, StepError> {
        ctx.check_cancelled()?;
        let source = ctx
            .source
            .ok_or_else(|| {
                KowalskiError::Validation("ingest: missing `source` in horde instruction".into())
            })?
            .to_string();
        let debug_root = ctx.workdir.join(WORKDIR_DEBUG_DIR);
        std::fs::create_dir_all(&debug_root).map_err(|e| {
            KowalskiError::Validation(format!("create {}: {e}", debug_root.display()))
        })?;
        let source_count = parse_input_assets(&source).len();
        let project_note = extract_project_path_from_source(&source)
            .map(|p| format!("; project tree from {}", p.display()))
            .unwrap_or_default();

        let path = tokio::task::spawn_blocking(move || {
            ingest_assets_markdown(&debug_root, &source).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| KowalskiError::Validation(format!("ingest task join: {e}")))?
        .map_err(KowalskiError::Validation)?;

        Ok(StepOutcome::Completed {
            summary: format!(
                "Captured {} source token(s) into raw collection: {}{}",
                source_count,
                path.display(),
                project_note
            ),
            artifact: Some(path),
            status: StageStatus::Pass,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(kind: &str, name: &str) -> StepSpec {
        StepSpec {
            name: name.into(),
            kind: kind.into(),
            output: Some(format!("debug/{name}.md")),
            ..Default::default()
        }
    }

    fn make_ctx<'a>(
        step: &'a StepSpec,
        workdir: &'a Path,
        source: Option<&'a str>,
        project_path: Option<PathBuf>,
        previous_artifact: Option<&'a Path>,
        cancel: &'a CancellationToken,
    ) -> StepContext<'a> {
        StepContext {
            run_id: "run-test",
            horde_id: "test-horde",
            step,
            workdir,
            horde_root: workdir,
            source,
            question: "q?",
            project_path,
            previous_artifact,
            events: &NullEventSink,
            llm: None,
            tools: None,
            cancel,
        }
    }

    #[test]
    fn registry_dispatch_by_kind() {
        let registry = StepHandlerRegistry::with_builtin_deterministic();
        assert_eq!(registry.kinds(), vec!["apply", "ingest", "verify"]);
        assert!(registry.contains("verify"));
        assert!(!registry.contains("process"), "LLM kinds stay on the worker path");
        assert_eq!(registry.get("apply").unwrap().kind(), "apply");
        assert!(registry.get("compile").is_none());
    }

    #[tokio::test]
    async fn verify_handler_writes_artifact_and_routes_pass_fail() {
        let dir = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        for (command, expected) in [("true", StageStatus::Pass), ("false", StageStatus::Fail)] {
            let mut step = spec("verify", "test-verify");
            step.verify_command = Some(command.into());
            let ctx = make_ctx(&step, dir.path(), None, Some(project.path().to_path_buf()), None, &cancel);
            let StepOutcome::Completed { artifact, status, summary } =
                VerifyStepHandler.execute(&ctx).await.unwrap();
            assert_eq!(status, expected, "command `{command}`");
            let artifact = artifact.unwrap();
            let body = std::fs::read_to_string(&artifact).unwrap();
            assert_eq!(
                crate::horde_stages::parse_stage_status_from_artifact(&body),
                Some(expected),
                "artifact frontmatter drives conditional edges"
            );
            assert!(summary.starts_with(&format!("verify `{}`", expected.as_str())));
        }
    }

    #[tokio::test]
    async fn verify_handler_requires_command_and_project() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let step = spec("verify", "test-verify");
        let ctx = make_ctx(&step, dir.path(), None, None, None, &cancel);
        let err = VerifyStepHandler.execute(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("verify_command"));
    }

    #[tokio::test]
    async fn apply_handler_dry_runs_previous_artifact() {
        let dir = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        // No ```diff blocks → nothing to apply → pass.
        let prev = dir.path().join("dev.md");
        std::fs::write(&prev, "plan only, no patches\n").unwrap();
        let step = spec("apply", "test-apply");
        let ctx = make_ctx(&step, dir.path(), None, Some(project.path().to_path_buf()), Some(&prev), &cancel);
        let StepOutcome::Completed { artifact, status, .. } =
            ApplyStepHandler.execute(&ctx).await.unwrap();
        assert_eq!(status, StageStatus::Pass);
        let body = std::fs::read_to_string(artifact.unwrap()).unwrap();
        assert!(body.contains("mode: dry-run"));

        // A diff that cannot apply → fail.
        std::fs::write(
            &prev,
            "```diff\n--- a/missing.rs\n+++ b/missing.rs\n@@ -1 +1 @@\n-x\n+y\n```\n",
        )
        .unwrap();
        let ctx = make_ctx(&step, dir.path(), None, Some(project.path().to_path_buf()), Some(&prev), &cancel);
        let StepOutcome::Completed { status, .. } = ApplyStepHandler.execute(&ctx).await.unwrap();
        assert_eq!(status, StageStatus::Fail);
    }

    #[tokio::test]
    async fn ingest_handler_captures_sources_under_debug() {
        let dir = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.rs"), "fn main() {}\n").unwrap();
        let cancel = CancellationToken::new();
        let step = spec("ingest", "test-ingest");
        let source = project.path().display().to_string();
        let ctx = make_ctx(&step, dir.path(), Some(&source), None, None, &cancel);
        let StepOutcome::Completed { artifact, status, summary } =
            IngestStepHandler.execute(&ctx).await.unwrap();
        assert_eq!(status, StageStatus::Pass);
        let artifact = artifact.unwrap();
        assert!(artifact.starts_with(dir.path().join(WORKDIR_DEBUG_DIR)));
        assert!(artifact.is_file());
        assert!(summary.contains("source token(s)"));
    }

    #[tokio::test]
    async fn cancelled_context_short_circuits() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let mut step = spec("verify", "test-verify");
        step.verify_command = Some("true".into());
        let ctx = make_ctx(&step, dir.path(), None, None, None, &cancel);
        let err = VerifyStepHandler.execute(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("cancelled"));
    }
}
