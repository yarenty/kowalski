//! Assemble **raw source markdown** from URLs, local file paths, and free text.
//!
//! Used by federation / agent-app **worker** runtimes. This module has **no** dependency on any
//! specific horde manifest — only paths you pass in (`root` / `workdir` for output layout).

use crate::operator_input::parse_operator_answer_block;
use crate::tools::internal::file_system::{self, DEFAULT_MAX_READ_BYTES};
use crate::tools::internal::github::{fetch_url_for_ingest, GithubFetchKind, resolve_github_fetch};
use crate::tools::internal::web::{fetch_url_as_markdown, looks_like_html};
use chrono::Utc;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceToken {
    Url(String),
    FilePath(String),
    Text(String),
}

/// One source the ingest could not read, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFailure {
    pub source: String,
    pub error: String,
}

/// What an ingest read and what it had to skip.
#[derive(Debug, Clone)]
pub struct IngestReport {
    /// The raw-collection markdown written under `root/raw/`.
    pub path: PathBuf,
    /// Sources whose content is in the collection (pages, files, prompt text, project tree).
    pub read: usize,
    /// Sources that could not be fetched or read.
    pub failed: Vec<SourceFailure>,
}

impl IngestReport {
    /// True when sources were named and none of them could be read: the collection holds only
    /// error notes, so nothing downstream has anything to work from.
    pub fn nothing_read(&self) -> bool {
        self.read == 0 && !self.failed.is_empty()
    }

    /// `source (reason); source (reason)` for run summaries and error messages.
    pub fn failed_summary(&self) -> String {
        self.failed
            .iter()
            .map(|f| format!("{} ({})", f.source, f.error))
            .collect::<Vec<_>>()
            .join("; ")
    }
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

fn normalize_fetched_url_body(text: &str, url: &str) -> String {
    crate::tools::internal::web::page_to_markdown(text, reqwest::Url::parse(url).ok().as_ref())
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
            let body = normalize_fetched_url_body(&fetched.text, url);
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

/// Limits when walking a local project directory for horde ingest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectWalkConfig {
    pub max_files: usize,
    pub max_total_bytes: usize,
    pub max_file_bytes: usize,
    pub max_depth: usize,
}

impl Default for ProjectWalkConfig {
    fn default() -> Self {
        Self {
            max_files: 200,
            max_total_bytes: 2 * 1024 * 1024,
            max_file_bytes: DEFAULT_MAX_READ_BYTES,
            max_depth: 16,
        }
    }
}

/// One row in a project tree walk (manifest + optional inline content).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectWalkEntry {
    pub rel_path: String,
    pub bytes: u64,
    pub included: bool,
    pub note: String,
}

fn skip_project_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | ".cursor" | "dist" | "build" | "__pycache__"
            | ".venv" | "venv" | "vendor" | "coverage" | ".next" | ".nuxt" | "tmp" | "output"
    ) || (name.starts_with('.') && name != ".")
}

fn skip_project_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cargo.lock"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
    ) || lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".ico")
        || lower.ends_with(".woff")
        || lower.ends_with(".woff2")
        || lower.ends_with(".ttf")
        || lower.ends_with(".eot")
        || lower.ends_with(".zip")
        || lower.ends_with(".tar")
        || lower.ends_with(".gz")
        || lower.ends_with(".bin")
        || lower.ends_with(".exe")
        || lower.ends_with(".dylib")
        || lower.ends_with(".so")
        || lower.ends_with(".dll")
        || lower.ends_with(".o")
        || lower.ends_with(".a")
}

/// Walk `root` recursively (breadth-first), skipping common build/cache dirs and binary blobs.
pub fn walk_project_tree(
    root: &Path,
    config: &ProjectWalkConfig,
) -> Result<Vec<ProjectWalkEntry>, String> {
    if !root.is_dir() {
        return Err(format!("project root is not a directory: {}", root.display()));
    }
    let root = file_system::try_canonicalize(root);
    let mut out = Vec::new();
    let mut total_bytes: usize = 0;
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.clone(), 0));

    while let Some((dir, depth)) = queue.pop_front() {
        if depth > config.max_depth {
            continue;
        }
        let entries = fs::read_dir(&dir).map_err(|e| {
            format!("read_dir {}: {}", dir.display(), e)
        })?;
        let mut names: Vec<_> = entries.flatten().collect();
        names.sort_by_key(|e| e.file_name());

        for entry in names {
            if out.len() >= config.max_files {
                out.push(ProjectWalkEntry {
                    rel_path: "(truncated)".into(),
                    bytes: 0,
                    included: false,
                    note: format!("max_files ({}) reached", config.max_files),
                });
                return Ok(out);
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if skip_project_dir(&name) {
                    continue;
                }
                queue.push_back((path, depth + 1));
                continue;
            }
            if !path.is_file() || skip_project_file(&name) {
                continue;
            }
            let rel = path
                .strip_prefix(&root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| name.clone());
            let bytes = file_system::file_len(&path).unwrap_or(0);
            let (included, note) = if bytes > config.max_file_bytes as u64 {
                (
                    false,
                    format!("skipped (file > {} bytes)", config.max_file_bytes),
                )
            } else if total_bytes.saturating_add(bytes as usize) > config.max_total_bytes {
                (
                    false,
                    format!(
                        "skipped (total budget {} bytes exhausted)",
                        config.max_total_bytes
                    ),
                )
            } else {
                total_bytes = total_bytes.saturating_add(bytes as usize);
                (true, "included".into())
            };
            out.push(ProjectWalkEntry {
                rel_path: rel,
                bytes,
                included,
                note,
            });
        }
    }
    Ok(out)
}

/// Resolve `project_path` from an operator prompt block (`answers_to_prompt` shape).
pub fn extract_project_path_from_source(source: &str) -> Option<PathBuf> {
    let answers = parse_operator_answer_block(source);
    for (label, value) in &answers {
        let ll = label.to_ascii_lowercase();
        if ll.contains("project path") || ll == "project_path" {
            let p = PathBuf::from(value.trim());
            if p.is_dir() {
                return Some(file_system::try_canonicalize(&p));
            }
        }
    }
    for token in source.split_whitespace() {
        let t = trim_token(token);
        if t.is_empty() {
            continue;
        }
        let p = Path::new(&t);
        if p.is_dir() && (p.is_absolute() || t.starts_with('.')) {
            return Some(file_system::try_canonicalize(p));
        }
    }
    None
}

fn render_project_tree_section(root: &Path, entries: &[ProjectWalkEntry]) -> String {
    let mut doc = String::new();
    doc.push_str("## Project tree\n\n");
    doc.push_str(&format!(
        "- Root: `{}`\n- Files indexed: {}\n- Files with inline content: {}\n\n",
        root.display(),
        entries.len(),
        entries.iter().filter(|e| e.included).count()
    ));
    doc.push_str("| Path | Bytes | Status |\n");
    doc.push_str("|---|---:|---|\n");
    for e in entries {
        if e.rel_path == "(truncated)" {
            doc.push_str(&format!(
                "| {} | — | {} |\n",
                md_cell(&e.rel_path),
                md_cell(&e.note)
            ));
            break;
        }
        doc.push_str(&format!(
            "| {} | {} | {} |\n",
            md_cell(&e.rel_path),
            e.bytes,
            md_cell(&e.note)
        ));
    }
    doc.push_str("\n### Included file contents\n\n");
    for e in entries.iter().filter(|e| e.included) {
        let full = root.join(&e.rel_path);
        let body = match file_system::read_file_bounded(&full, DEFAULT_MAX_READ_BYTES) {
            Ok(s) => s.chars().take(24000).collect::<String>(),
            Err(err) => format!("(unable to read: {err})"),
        };
        doc.push_str(&format!(
            "<!-- project-file:{}:begin -->\n#### `{}`\n\n{}\n\n<!-- project-file:{}:end -->\n\n",
            e.rel_path, e.rel_path, body, e.rel_path
        ));
    }
    doc
}

/// Write `raw/<stamp>-inputs-N.md` under `root` from mixed URL / file / text input.
///
/// `root` is typically `workdir/debug`; bundled markdown lands in `workdir/debug/raw/`.
pub fn write_raw_sources_markdown(
    root: &Path,
    source_input: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    ingest_sources(root, source_input).map(|report| report.path)
}

/// [`write_raw_sources_markdown`] plus the count of sources read and the list of those skipped,
/// so the caller can fail a run that has nothing to work from.
pub fn ingest_sources(
    root: &Path,
    source_input: &str,
) -> Result<IngestReport, Box<dyn std::error::Error>> {
    let assets = parse_source_tokens(source_input);
    let project_root = extract_project_path_from_source(source_input);
    let project_entries = project_root
        .as_ref()
        .map(|r| walk_project_tree(r, &ProjectWalkConfig::default()))
        .transpose()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let asset_count = assets.len()
        + usize::from(project_entries.as_ref().is_some_and(|e| !e.is_empty()));
    let out = root
        .join("raw")
        .join(format!("{stamp}-inputs-{asset_count}.md"));
    let now = Utc::now().to_rfc3339();
    let mut doc = String::new();
    doc.push_str("# Raw Inputs\n\n");
    doc.push_str(&format!(
        "- Inputs: {}\n- Ingested At: {}\n\n",
        asset_count,
        now
    ));
    if let Some(root) = &project_root {
        doc.push_str(&format!("- Project root: `{}`\n", root.display()));
    }
    doc.push_str("## Sources Metadata\n\n");
    doc.push_str("| # | Type | Source | Status | Chars | Notes |\n");
    doc.push_str("|---:|---|---|---|---:|---|\n");

    let mut sections = String::new();
    let mut read = usize::from(project_entries.as_ref().is_some_and(|e| !e.is_empty()));
    let mut failed: Vec<SourceFailure> = Vec::new();

    for (idx, asset) in assets.iter().enumerate() {
        match asset {
            SourceToken::Url(url) => {
                let section = match fetch_url_for_bundle(url) {
                    Ok((body, note, resolved)) => {
                        read += 1;
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
                        failed.push(SourceFailure { source: url.clone(), error: e.clone() });
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
                let (content, status) = match file_system::read_file_bounded(Path::new(path), DEFAULT_MAX_READ_BYTES)
                {
                    Ok(s) => {
                        read += 1;
                        (s, "ok")
                    }
                    Err(e) => {
                        failed.push(SourceFailure { source: path.clone(), error: e.to_string() });
                        (format!("(unable to read file: {e})"), "error")
                    }
                };
                let clipped = content.chars().take(24000).collect::<String>();
                doc.push_str(&format!(
                    "| {} | file | {} | {} | {} | local file |\n",
                    idx + 1,
                    path,
                    status,
                    if status == "ok" { clipped.chars().count() } else { 0 }
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
                read += 1;
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
    if !failed.is_empty() {
        doc.push_str("## Skipped Sources\n\n");
        doc.push_str(
            "These sources could not be read. Nothing below comes from them; say so instead of guessing their content.\n\n",
        );
        for f in &failed {
            doc.push_str(&format!("- `{}`: {}\n", f.source, f.error));
        }
        doc.push('\n');
    }
    doc.push_str("## Source Collection\n\n");
    if let (Some(root), Some(entries)) = (project_root.as_ref(), project_entries.as_ref()) {
        doc.push_str(&render_project_tree_section(root, entries));
    }
    doc.push_str(&sections);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&out, doc)?;
    Ok(IngestReport { path: out, read, failed })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn walk_skips_git_and_target() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
        fs::create_dir_all(dir.path().join("target/debug")).unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("README.md"), "# hi").unwrap();
        let entries = walk_project_tree(dir.path(), &ProjectWalkConfig::default()).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.rel_path.as_str()).collect();
        assert!(paths.contains(&"src/lib.rs"));
        assert!(paths.contains(&"README.md"));
        assert!(!paths.iter().any(|p| p.contains(".git")));
        assert!(!paths.iter().any(|p| p.contains("target")));
    }

    /// Nothing listens on port 1, so the fetch is refused at once with no network needed.
    const DEAD_URL: &str = "http://127.0.0.1:1/feed.xml";

    #[test]
    fn ingest_reports_nothing_read_when_every_source_fails() {
        let out = tempfile::tempdir().unwrap();
        let input = format!("{DEAD_URL} http://127.0.0.1:1/other.xml");
        let report = ingest_sources(out.path(), &input).unwrap();
        assert_eq!(report.read, 0);
        assert_eq!(report.failed.len(), 2);
        assert!(report.nothing_read());
        assert!(report.failed_summary().contains(DEAD_URL));
        let body = fs::read_to_string(&report.path).unwrap();
        assert!(body.contains("## Skipped Sources"));
        assert!(body.contains("| error |"));
    }

    #[test]
    fn ingest_names_skipped_sources_when_some_are_read() {
        let out = tempfile::tempdir().unwrap();
        let notes = tempfile::tempdir().unwrap();
        let file = notes.path().join("notes.md");
        fs::write(&file, "alpha beta").unwrap();
        let input = format!("{DEAD_URL} {}", file.display());
        let report = ingest_sources(out.path(), &input).unwrap();
        assert_eq!(report.read, 1);
        assert_eq!(report.failed.len(), 1);
        assert!(!report.nothing_read());
        let body = fs::read_to_string(&report.path).unwrap();
        assert!(body.contains("alpha beta"));
        assert!(body.contains(&format!("- `{DEAD_URL}`")));
    }

    #[test]
    fn ingest_of_plain_text_reads_one_source_and_skips_none() {
        let out = tempfile::tempdir().unwrap();
        let report = ingest_sources(out.path(), "summarise the quarter").unwrap();
        assert_eq!(report.read, 1);
        assert!(report.failed.is_empty());
        assert!(!fs::read_to_string(&report.path).unwrap().contains("Skipped Sources"));
    }

    #[test]
    fn extract_project_path_from_operator_block() {
        let block = "# Operator input (Project intake)\n\n**Project path (local folder or repo root):** /tmp/kowalski\n\n**Task specification:** fix bugs\n";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_string_lossy().to_string();
        let block = block.replace("/tmp/kowalski", &path);
        let got = extract_project_path_from_source(&block).unwrap();
        assert_eq!(got, file_system::try_canonicalize(dir.path()));
    }

    #[test]
    fn intake_includes_project_tree() {
        let work = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("main.rs"), "fn main() {}").unwrap();
        let source = format!(
            "# Operator input\n\n**Project path (local folder or repo root):** {}\n\n**Task:** ship it\n",
            project.path().display()
        );
        let out = write_raw_sources_markdown(work.path(), &source).unwrap();
        let body = fs::read_to_string(out).unwrap();
        assert!(body.contains("## Project tree"));
        assert!(body.contains("main.rs"));
        assert!(body.contains("fn main()"));
    }
}
