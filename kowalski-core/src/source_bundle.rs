//! Assemble **raw source markdown** from URLs, local file paths, and free text.
//!
//! Used by federation / agent-app **worker** runtimes. This module has **no** dependency on any
//! specific horde manifest — only paths you pass in (`root` / `workdir` for output layout).

use crate::tools::internal::file_system::{self, DEFAULT_MAX_READ_BYTES};
use crate::tools::internal::github::{fetch_url_for_ingest, GithubFetchKind, resolve_github_fetch};
use crate::tools::internal::web::{fetch_url_as_markdown, html_body_to_markdown, looks_like_html};
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceToken {
    Url(String),
    FilePath(String),
    Text(String),
}

fn trim_token(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '(' || c == ')')
        .trim_end_matches([',', ';', ':', '.'])
        .to_string()
}

/// Split CLI-style input into URL / existing file / fallback text.
pub fn parse_source_tokens(input: &str) -> Vec<SourceToken> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for token in input.split_whitespace() {
        let t = trim_token(token);
        if t.is_empty() {
            continue;
        }
        let asset = if t.starts_with("http://") || t.starts_with("https://") {
            SourceToken::Url(t.clone())
        } else {
            let p = Path::new(&t);
            if p.is_file() {
                SourceToken::FilePath(t.clone())
            } else {
                continue;
            }
        };
        let key = format!("{:?}", asset);
        if seen.insert(key) {
            out.push(asset);
        }
    }
    if out.is_empty() {
        let t = input.trim();
        if !t.is_empty() {
            out.push(SourceToken::Text(t.to_string()));
        }
    }
    out
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn md_cell(input: &str) -> String {
    input.replace('|', "\\|").replace('\n', " ")
}

fn normalize_fetched_url_body(text: &str) -> String {
    if looks_like_html(text) {
        html_body_to_markdown(text)
    } else {
        text.to_string()
    }
}

/// **GitHub.com URLs** → [`fetch_url_for_ingest`](crate::tools::internal::github::fetch_url_for_ingest) (README API / raw / token).
/// **All other HTTP(S) URLs** → [`fetch_url_as_markdown`](crate::tools::internal::web::fetch_url_as_markdown) (GET + HTML→MD when needed).
/// If GitHub-specific fetch fails, falls back once to the web path.
fn fetch_url_for_bundle(url: &str) -> Result<(String, String, String), String> {
    let github_shape = resolve_github_fetch(url).is_some();
    if github_shape
        && let Ok(fetched) = fetch_url_for_ingest(url)
    {
            let via = match fetched.kind {
                GithubFetchKind::ReadmeApi => "github readme api",
                GithubFetchKind::RawUserContent => "github raw",
                GithubFetchKind::PlainHttp => "github plain http",
            };
            let body = normalize_fetched_url_body(&fetched.text);
            let note = if looks_like_html(&fetched.text) {
                format!("{via}; html→md")
            } else {
                via.to_string()
            };
            return Ok((body, note, fetched.resolved_url));
    }
    let body = fetch_url_as_markdown(url).map_err(|e| e.to_string())?;
    let note = if github_shape {
        "web fetch (GitHub ingest failed or non-API body)".to_string()
    } else {
        "web fetch (non-GitHub URL)".to_string()
    };
    Ok((body, note, url.to_string()))
}

/// Write `raw/<stamp>-inputs-N.md` under `root` from mixed URL / file / text input.
///
/// `root` is typically `workdir/debug`; bundled markdown lands in `workdir/debug/raw/`.
pub fn write_raw_sources_markdown(
    root: &Path,
    source_input: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let assets = parse_source_tokens(source_input);
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let out = root
        .join("raw")
        .join(format!("{stamp}-inputs-{}.md", assets.len()));
    let now = Utc::now().to_rfc3339();
    let mut doc = String::new();
    doc.push_str("# Raw Inputs\n\n");
    doc.push_str(&format!(
        "- Inputs: {}\n- Ingested At: {}\n\n",
        assets.len(),
        now
    ));
    doc.push_str("## Sources Metadata\n\n");
    doc.push_str("| # | Type | Source | Status | Chars | Notes |\n");
    doc.push_str("|---:|---|---|---|---:|---|\n");

    let mut sections = String::new();

    for (idx, asset) in assets.iter().enumerate() {
        match asset {
            SourceToken::Url(url) => {
                let section = match fetch_url_for_bundle(url) {
                    Ok((body, note, resolved)) => {
                        let clipped = body.chars().take(24000).collect::<String>();
                        doc.push_str(&format!(
                            "| {} | url | {} | ok | {} | {} |\n",
                            idx + 1,
                            md_cell(url),
                            clipped.chars().count(),
                            md_cell(&note),
                        ));
                        format!(
                            "<!-- source:{}:url:begin -->\n## Source {}: URL\n\n- Original URL: `{}`\n- Resolved / peer: `{}`\n- Mode: {}\n\n{}\n\n<!-- source:{}:url:end -->\n\n",
                            idx + 1,
                            idx + 1,
                            url,
                            resolved,
                            note,
                            clipped,
                            idx + 1
                        )
                    }
                    Err(e) => {
                        let err = format!("Fetch error: {}", e);
                        doc.push_str(&format!(
                            "| {} | url | {} | error | 0 | {} |\n",
                            idx + 1,
                            md_cell(url),
                            md_cell(&err)
                        ));
                        format!(
                            "<!-- source:{}:url:begin -->\n## Source {}: URL\n\n- URL: `{}`\n\n{}\n\n<!-- source:{}:url:end -->\n\n",
                            idx + 1,
                            idx + 1,
                            url,
                            err,
                            idx + 1
                        )
                    }
                };
                sections.push_str(&section);
            }
            SourceToken::FilePath(path) => {
                let content = match file_system::read_file_bounded(Path::new(path), DEFAULT_MAX_READ_BYTES)
                {
                    Ok(s) => s,
                    Err(e) => format!("(unable to read file: {e})"),
                };
                let clipped = content.chars().take(24000).collect::<String>();
                doc.push_str(&format!(
                    "| {} | file | {} | ok | {} | local file |\n",
                    idx + 1,
                    path,
                    clipped.chars().count()
                ));
                sections.push_str(&format!(
                    "<!-- source:{}:file:begin -->\n## Source {}: File\n\n- Path: `{}`\n\n{}\n\n<!-- source:{}:file:end -->\n\n",
                    idx + 1,
                    idx + 1,
                    path,
                    clipped,
                    idx + 1
                ));
            }
            SourceToken::Text(text) => {
                let slug = slugify(text);
                doc.push_str(&format!(
                    "| {} | text | {} | ok | {} | direct prompt text |\n",
                    idx + 1,
                    md_cell(if slug.is_empty() { "prompt" } else { &slug }),
                    text.chars().count()
                ));
                sections.push_str(&format!(
                    "<!-- source:{}:text:begin -->\n## Source {}: Text\n\n- Label: `{}`\n\n{}\n\n<!-- source:{}:text:end -->\n\n",
                    idx + 1,
                    idx + 1,
                    if slug.is_empty() { "prompt" } else { &slug },
                    text,
                    idx + 1
                ));
            }
        }
    }
    doc.push('\n');
    doc.push_str("## Source Collection\n\n");
    doc.push_str(&sections);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, doc)?;
    Ok(out)
}

/// Alias for older naming (`InputAsset` in the CLI crate).
pub type InputAsset = SourceToken;

/// Back-compat: same as [`parse_source_tokens`].
pub fn parse_input_assets(input: &str) -> Vec<InputAsset> {
    parse_source_tokens(input)
}

/// Back-compat: same as [`write_raw_sources_markdown`].
pub fn ingest_assets_markdown(
    root: &Path,
    source_input: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    write_raw_sources_markdown(root, source_input)
}
