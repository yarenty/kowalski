//! Markdown-staged **apps**: a manifest (`app.md` or `horde.md`) plus `agents/*.md` drive
//! multi-step runs. This module is **layout-agnostic** — no wiki/Obsidian-specific repair or
//! index generation. Each stage declares prompt paths, optional context files (relative to the
//! workdir), optional `@artifact@` / `@step:name@` tokens, and optional markdown normalization.

use crate::error::KowalskiError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Top-level manifest (TOML frontmatter in `app.md` or `horde.md`).
#[derive(Debug, Deserialize, Clone)]
pub struct AppManifestMeta {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub pipeline: Vec<String>,
    #[serde(default)]
    pub default_question: Option<String>,
}

/// One pipeline stage (`agents/<name>.md` frontmatter).
#[derive(Debug, Deserialize, Clone)]
pub struct StageAgentMeta {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub prompt_file: Option<String>,
    /// Relative to **workdir** (e.g. `debug/reports/latest.md`). Required for LLM stages.
    #[serde(default)]
    pub output: Option<String>,
    /// Paths relative to **workdir**, and/or:
    /// - `@artifact@` — previous stage artifact path (federation / chained runs),
    /// - `@step:<pipeline_step_name>@` — output path recorded for that step (local runner).
    #[serde(default)]
    pub context_paths: Vec<String>,
    #[serde(default)]
    pub normalize_doc_title: Option<String>,
    #[serde(default)]
    pub normalize_sections: Vec<String>,
    #[serde(default)]
    pub normalize_fallback: Option<String>,
    /// Section headings (## name) that receive `normalize_fallback` body when synthesized.
    #[serde(default)]
    pub normalize_fallback_sections: Vec<String>,
}

/// Prefer `app.md`, then legacy `horde.md`.
pub fn resolve_manifest_path(app_root: &Path) -> PathBuf {
    let app_md = app_root.join("app.md");
    if app_md.is_file() {
        return app_md;
    }
    app_root.join("horde.md")
}

pub fn parse_app_manifest(path: &Path) -> Result<AppManifestMeta, KowalskiError> {
    parse_md_frontmatter(path)
}

pub fn parse_stage_agent(path: &Path) -> Result<StageAgentMeta, KowalskiError> {
    parse_md_frontmatter(path)
}

pub fn load_stage_agents(agents_dir: &Path) -> Result<BTreeMap<String, StageAgentMeta>, KowalskiError> {
    let mut map = BTreeMap::new();
    let rd = fs::read_dir(agents_dir)
        .map_err(|e| KowalskiError::Validation(format!("read agents dir {}: {}", agents_dir.display(), e)))?;
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let doc: StageAgentMeta = parse_stage_agent(&p)?;
        map.insert(doc.name.clone(), doc);
    }
    Ok(map)
}

fn parse_md_frontmatter<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, KowalskiError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| KowalskiError::Validation(format!("read {}: {}", path.display(), e)))?;
    let mut lines = raw.lines();
    if lines.next().map(|s| s.trim()) != Some("---") {
        return Err(KowalskiError::Validation(format!(
            "missing frontmatter start in {}",
            path.display()
        )));
    }
    let mut fm = String::new();
    let mut in_fm = true;
    for line in raw.lines().skip(1) {
        if in_fm && line.trim() == "---" {
            in_fm = false;
            break;
        }
        if in_fm {
            fm.push_str(line);
            fm.push('\n');
        }
    }
    if in_fm {
        return Err(KowalskiError::Validation(format!(
            "missing frontmatter end in {}",
            path.display()
        )));
    }
    toml::from_str::<T>(&fm).map_err(|e| {
        KowalskiError::Validation(format!("toml in {}: {}", path.display(), e))
    })
}

fn resolve_path_token(
    workdir: &Path,
    token: &str,
    step_paths: &BTreeMap<String, PathBuf>,
    previous_artifact: Option<&Path>,
) -> Result<PathBuf, KowalskiError> {
    let t = token.trim();
    if t == "@artifact@" {
        return previous_artifact
            .map(|p| p.to_path_buf())
            .ok_or_else(|| KowalskiError::Validation("@artifact@ used but no previous artifact".into()));
    }
    if let Some(inner) = t.strip_prefix("@step:").and_then(|s| s.strip_suffix("@")) {
        let name = inner.trim();
        return step_paths.get(name).cloned().ok_or_else(|| {
            KowalskiError::Validation(format!("@step:{name}@ not available (no output for that stage)"))
        });
    }
    let p = workdir.join(t);
    if p.is_file() {
        Ok(p)
    } else {
        Err(KowalskiError::Validation(format!(
            "context path not found: {}",
            p.display()
        )))
    }
}

/// Concatenate resolved context files as markdown sections for LLM consumption.
pub fn render_context_attachments(
    workdir: &Path,
    tokens: &[String],
    step_paths: &BTreeMap<String, PathBuf>,
    previous_artifact: Option<&Path>,
) -> Result<String, KowalskiError> {
    let mut out = String::new();
    for token in tokens {
        let path = resolve_path_token(workdir, token, step_paths, previous_artifact)?;
        let label = path.strip_prefix(workdir).unwrap_or(&path).display().to_string();
        let body = fs::read_to_string(&path).unwrap_or_default();
        out.push_str(&format!("## Context file: `{label}`\n\n{body}\n\n---\n\n"));
    }
    Ok(out)
}

/// Optional markdown normalization (H1 + required ## sections).
pub fn maybe_normalize_markdown(agent: &StageAgentMeta, raw: &str) -> String {
    let Some(ref title) = agent.normalize_doc_title else {
        return raw.trim().to_string();
    };
    if agent.normalize_sections.is_empty() {
        return raw.trim().to_string();
    }
    let sections: Vec<String> = agent.normalize_sections.clone();
    let sec_refs: Vec<&str> = sections.iter().map(String::as_str).collect();
    let fallback = agent
        .normalize_fallback
        .as_deref()
        .unwrap_or("Fallback model output was empty or unusable.");
    let fallback_for: Vec<&str> = if agent.normalize_fallback_sections.is_empty() {
        vec!["Summary", "Response", "Issues", "Snapshot"]
    } else {
        agent
            .normalize_fallback_sections
            .iter()
            .map(String::as_str)
            .collect()
    };
    normalize_markdown_sections(raw, title, &sec_refs, fallback, &fallback_for)
}

fn normalize_markdown_sections(
    raw: &str,
    title: &str,
    required_sections: &[&str],
    fallback_body: &str,
    fallback_for: &[&str],
) -> String {
    let trimmed = raw.trim();
    let mut out = String::new();
    if trimmed.is_empty() || trimmed == "{}" || trimmed == "null" {
        out.push_str(&format!("# {}\n\n", title));
        for s in required_sections {
            out.push_str(&format!("## {}\n", s));
            if fallback_for.contains(s) {
                out.push_str(fallback_body);
                out.push('\n');
            }
            out.push('\n');
        }
        return out;
    }

    let mut body = trimmed.to_string();
    // If there is no top-level H1 (`# `…), prefix the manifest title. Do **not** append
    // synthetic `##` sections here: models often use emoji or alternate headings (e.g.
    // `## 📝 TL;DR` instead of `## TL;DR`), and exact `contains("## TL;DR")` checks falsely
    // inject duplicate sections plus `normalize_fallback` text into otherwise valid output.
    let start = body.trim_start();
    if !start.starts_with("# ") {
        body = format!("# {}\n\n{}", title, start);
    }
    body.push('\n');
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lint_like_agent() -> StageAgentMeta {
        StageAgentMeta {
            name: "lint".into(),
            kind: "lint".into(),
            prompt_file: None,
            output: None,
            context_paths: vec![],
            normalize_doc_title: Some("Vault paste pack".into()),
            normalize_sections: vec!["TL;DR".into(), "Suggested notes".into()],
            normalize_fallback: Some("SHOULD_NOT_APPEAR".into()),
            normalize_fallback_sections: vec!["TL;DR".into()],
        }
    }

    #[test]
    fn maybe_normalize_non_empty_does_not_inject_fallback_for_emoji_headings() {
        let agent = lint_like_agent();
        let raw = "## 📝 TL;DR\n\nReal content.\n\n## 🔗 Links\n\nMore.\n";
        let out = maybe_normalize_markdown(&agent, raw);
        assert!(
            !out.contains("SHOULD_NOT_APPEAR"),
            "unexpected fallback injection: {}",
            out
        );
        assert!(out.contains("Real content."));
        assert!(out.contains("Vault paste pack"));
    }

    #[test]
    fn maybe_normalize_empty_still_synthesizes_with_fallback() {
        let agent = lint_like_agent();
        let out = maybe_normalize_markdown(&agent, "  ");
        assert!(out.contains("SHOULD_NOT_APPEAR"));
        assert!(out.contains("# Vault paste pack"));
    }
}
