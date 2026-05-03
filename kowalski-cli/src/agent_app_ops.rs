//! Markdown-defined app agent orchestration (`horde.md` pipeline + `agents/*.md`).

use kowalski_core::source_bundle::{ingest_assets_markdown, parse_input_assets};
use chrono::Utc;
use reqwest::blocking as reqwest_blocking;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// Intermediate artifacts (ingest, wiki, reports) live here — for monitoring / debugging only.
const ARTIFACTS_DEBUG_DIR: &str = "debug";
/// Single operator-facing markdown at the `workdir` (or app run) root.
const PASTE_ME_FILENAME: &str = "PASTE_ME.md";

#[inline]
fn work_debug(workdir: &Path) -> PathBuf {
    workdir.join(ARTIFACTS_DEBUG_DIR)
}

/// Default workdir for local `agent-app run` (matches `horde.md` `workdir = "output"` in the KC example).
const LOCAL_AGENT_APP_WORKDIR: &str = "output";

#[inline]
fn local_agent_app_workdir(app_root: &Path) -> PathBuf {
    app_root.join(LOCAL_AGENT_APP_WORKDIR)
}

/// Subset of `horde.md` frontmatter consumed by `agent-app` (extra keys are ignored).
#[derive(Debug, Deserialize)]
struct HordeManifestMeta {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    pipeline: Vec<String>,
    #[serde(default)]
    default_question: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubAgentMeta {
    name: String,
    kind: String,
    prompt_file: Option<String>,
    output: Option<String>,
}

#[derive(Default, Clone)]
struct RunArtifacts {
    summary: Option<PathBuf>,
    report: Option<PathBuf>,
    lint: Option<PathBuf>,
    log: Option<PathBuf>,
}

#[derive(Debug)]
struct AgentDoc<T> {
    meta: T,
    path: PathBuf,
}

type AgentSpecMap = BTreeMap<String, AgentDoc<SubAgentMeta>>;

fn parse_markdown_with_toml_frontmatter<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<AgentDoc<T>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let mut lines = raw.lines();
    if lines.next().map(|s| s.trim()) != Some("---") {
        return Err(format!("Missing frontmatter start in {}", path.display()).into());
    }
    let mut fm = String::new();
    let mut in_fm = true;
    for line in raw.lines().skip(1) {
        if in_fm && line.trim() == "---" {
            in_fm = false;
            continue;
        }
        if in_fm {
            fm.push_str(line);
            fm.push('\n');
        }
    }
    if in_fm {
        return Err(format!("Missing frontmatter end in {}", path.display()).into());
    }
    let meta: T = toml::from_str(&fm)?;
    Ok(AgentDoc {
        meta,
        path: path.to_path_buf(),
    })
}

fn app_root(path: Option<&str>) -> PathBuf {
    path.map(PathBuf::from)
        .or_else(|| std::env::var("KOWALSKI_AGENT_APP_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("examples/knowledge-compiler"))
}

fn horde_manifest_path(root: &Path) -> PathBuf {
    root.join("horde.md")
}

fn agents_dir(root: &Path) -> PathBuf {
    root.join("agents")
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

fn load_spec(
    root: &Path,
) -> Result<(AgentDoc<HordeManifestMeta>, AgentSpecMap), Box<dyn std::error::Error>> {
    let hpath = horde_manifest_path(root);
    if !hpath.is_file() {
        return Err(format!(
            "missing {} — agent-app expects horde.md next to agents/",
            hpath.display()
        )
        .into());
    }
    let main = parse_markdown_with_toml_frontmatter::<HordeManifestMeta>(&hpath)?;
    let mut map = BTreeMap::new();
    for entry in fs::read_dir(agents_dir(root))? {
        let path = entry?.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let doc = parse_markdown_with_toml_frontmatter::<SubAgentMeta>(&path)?;
        map.insert(doc.meta.name.clone(), doc);
    }
    Ok((main, map))
}

fn collect_md_paths_budgeted(doc_dir: &Path, max_files: usize) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_md_recursive_paths_cap(doc_dir, doc_dir, &mut paths, max_files)?;
    paths.sort();
    Ok(paths)
}

fn collect_md_recursive_paths_cap(
    dir: &Path,
    doc_root: &Path,
    out: &mut Vec<PathBuf>,
    max_files: usize,
) -> std::io::Result<()> {
    if out.len() >= max_files {
        return Ok(());
    }
    for e in fs::read_dir(dir)? {
        if out.len() >= max_files {
            break;
        }
        let p = e?.path();
        if p.is_dir() {
            collect_md_recursive_paths_cap(&p, doc_root, out, max_files)?;
        } else if p.extension().and_then(|x| x.to_str()) == Some("md") {
            if p.strip_prefix(doc_root).is_ok() {
                out.push(p);
            }
        }
    }
    Ok(())
}

const OBSIDIAN_PLACEMENT_TIP: &str = r#"## Where to file this (suggestion)

Create **one new note** in your vault and paste everything **below the horizontal rule**.

Many people keep a topic tree similar to an mdBook `doc/` layout, for example:

```text
programming/
  assembly.md
  package_managers.md
  rust/
    borrow_check.md
    concurrency.md
    README.md
    ...
```

Your vault might use `Areas/Programming/Rust/` or an `Inbox/` — any folder is fine; adjust `[[wikilinks]]` to match your scheme.
"#;

fn extract_h2_section(raw: &str, title: &str) -> String {
    let needle = format!("## {}", title);
    let Some(idx) = raw.find(&needle) else {
        return String::new();
    };
    let after = &raw[idx + needle.len()..];
    let after = after.trim_start();
    let end = after
        .find("\n## ")
        .or_else(|| after.find("\r\n## "))
        .unwrap_or(after.len());
    after[..end].trim().to_string()
}

/// `workdir/PASTE_ME.md` — copy-paste into a personal note (e.g. Obsidian). Written at end of lint.
fn write_paste_me_file(
    workdir: &Path,
    source_hint: Option<&str>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dbg = work_debug(workdir);
    let summary = read_or_empty(&dbg.join("wiki/summaries/latest.md"));
    let report = read_or_empty(&dbg.join("derived/reports/latest.md"));
    let lint = read_or_empty(&dbg.join("derived/lint/latest.md"));
    let sum_sec = extract_h2_section(&summary, "Summary");
    let concepts_sec = extract_h2_section(&summary, "Extracted Concepts");
    let resp_sec = extract_h2_section(&report, "Response");
    let q_sec = extract_h2_section(&report, "Question");
    let lint_snap = extract_h2_section(&lint, "Snapshot");
    let stamp = Utc::now().to_rfc3339();

    let mut out = String::from("# Paste into Obsidian\n\n");
    out.push_str(OBSIDIAN_PLACEMENT_TIP);
    out.push_str("\n\n---\n\n");
    if let Some(s) = source_hint.map(str::trim).filter(|s| !s.is_empty()) {
        out.push_str("## Source\n\n");
        out.push_str(s);
        out.push_str("\n\n---\n\n");
    }
    if !q_sec.is_empty() {
        out.push_str("## Question\n\n");
        out.push_str(&q_sec);
        out.push_str("\n\n---\n\n");
    }
    if !sum_sec.is_empty() {
        out.push_str("## Summary (from compile)\n\n");
        out.push_str(&sum_sec);
        out.push_str("\n\n---\n\n");
    }
    if !concepts_sec.is_empty() {
        out.push_str("## Extracted concepts\n\n");
        out.push_str(&concepts_sec);
        out.push_str("\n\n---\n\n");
    }
    if !resp_sec.is_empty() {
        out.push_str("## Answer (from ask)\n\n");
        out.push_str(&resp_sec);
        out.push_str("\n\n---\n\n");
    }
    if !lint_snap.is_empty() {
        out.push_str("## Lint snapshot\n\n");
        out.push_str(&lint_snap);
        out.push_str("\n\n---\n\n");
    }
    out.push_str("## Meta\n\n");
    out.push_str(&format!(
        "- Generated: `{}`\n- Intermediate files (raw ingest, wiki, reports): `{}/`\n",
        stamp,
        ARTIFACTS_DEBUG_DIR
    ));
    fs::create_dir_all(workdir)?;
    let path = workdir.join(PASTE_ME_FILENAME);
    fs::write(&path, out)?;
    Ok(path)
}

pub fn list_agents(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    let (main, agents) = load_spec(&root)?;
    let title = main
        .meta
        .display_name
        .as_deref()
        .unwrap_or(&main.meta.id);
    println!("Horde: {} ({})", title, main.meta.id);
    println!("Pipeline: {}", main.meta.pipeline.join(" -> "));
    println!("Pipeline agents:");
    for step in &main.meta.pipeline {
        if let Some(agent) = agents.get(step) {
            println!("- {} ({})", step, agent.meta.kind);
        } else {
            println!("- {} (missing agents/{step}.md)", step);
        }
    }
    for name in agents.keys() {
        if !main.meta.pipeline.contains(name) {
            if let Some(agent) = agents.get(name) {
                println!(
                    "- {} ({}) [not in horde.md pipeline — remove or add to pipeline]",
                    name, agent.meta.kind
                );
            }
        }
    }
    Ok(())
}

pub fn validate(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    let (main, agents) = load_spec(&root)?;
    let mut errs = Vec::new();
    let defs: BTreeSet<_> = agents.keys().cloned().collect();
    let pipeline_set: BTreeSet<_> = main.meta.pipeline.iter().cloned().collect();

    for name in &main.meta.pipeline {
        if !defs.contains(name) {
            errs.push(format!(
                "horde.md pipeline references missing agent definition `{name}` (expected agents/{name}.md)"
            ));
        }
    }
    for name in &defs {
        if !pipeline_set.contains(name) {
            errs.push(format!(
                "agents/{name}.md exists but `{name}` is not listed in horde.md pipeline"
            ));
        }
    }
    for (name, agent) in &agents {
        if agent.meta.name != *name {
            errs.push(format!(
                "agent name mismatch in {} (key `{}` vs meta `{}`)",
                agent.path.display(),
                name,
                agent.meta.name
            ));
        }
    }

    if errs.is_empty() {
        println!("OK - horde.md + agents/ definitions are valid");
        return Ok(());
    }
    for e in errs {
        eprintln!("ERROR: {}", e);
    }
    Err("horde.md + agents/ definition invalid".into())
}

fn chat_no_tools(api: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest_blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;
    let route = "/api/chat";
    let url = format!("{}{}", api.trim_end_matches('/'), route);
    let resp = client
        .post(format!("{}/api/chat", api.trim_end_matches('/')))
        .json(&serde_json::json!({
            "message": prompt,
            "use_memory": false,
            "use_tools": false
        }))
        .send()
        .map_err(|e| friendly_http_error(api, route, &url, &e))?;
    if !resp.status().is_success() {
        return Err(
            friendly_http_status_error(api, route, &url, resp.status().as_u16(), None).into(),
        );
    }
    let v: serde_json::Value = resp.json()?;
    Ok(v.get("reply")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim()
        .to_string())
}

fn read_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn ensure_dirs(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(root)?;
    let b = work_debug(root);
    for rel in [
        "raw/sources",
        "raw/images",
        "wiki/concepts",
        "wiki/summaries",
        "derived/reports",
        "derived/lint",
        "scratch",
    ] {
        fs::create_dir_all(b.join(rel))?;
    }
    Ok(())
}

fn normalize_markdown_sections(
    raw: &str,
    title: &str,
    required_sections: &[&str],
    fallback_body: &str,
) -> String {
    let trimmed = raw.trim();
    let mut out = String::new();
    if trimmed.is_empty() || trimmed == "{}" || trimmed == "null" {
        out.push_str(&format!("# {}\n\n", title));
        for s in required_sections {
            out.push_str(&format!("## {}\n", s));
            if *s == "Summary" || *s == "Response" || *s == "Issues" {
                out.push_str(fallback_body);
                out.push('\n');
            }
            out.push('\n');
        }
        return out;
    }

    let mut body = trimmed.to_string();
    if !body.starts_with("# ") {
        body = format!("# {}\n\n{}", title, body);
    }
    for s in required_sections {
        let marker = format!("## {}", s);
        if !body.contains(&marker) {
            body.push_str(&format!("\n\n{}\n", marker));
            if *s == "Summary" || *s == "Response" || *s == "Issues" {
                body.push_str(fallback_body);
                body.push('\n');
            }
        }
    }
    body.push('\n');
    body
}

fn wikilink_path_for_wiki_root(wiki_root: &Path, md_file: &Path) -> Option<String> {
    let rel = md_file.strip_prefix(wiki_root).ok()?;
    let mut s = rel.to_string_lossy().replace('\\', "/");
    if s.ends_with(".md") {
        s.truncate(s.len().saturating_sub(3));
    }
    if s.is_empty() || s == "index" {
        return None;
    }
    Some(format!("- [[{s}]]"))
}

/// Top-level folders under `wiki/` other than `concepts` / `summaries` (e.g. hand-placed reference trees) for Obsidian MOC.
fn list_bundled_reference_lines(wiki_root: &Path) -> Vec<String> {
    let skip: BTreeSet<&str> = BTreeSet::from(["concepts", "summaries"]);
    let mut lines = Vec::new();
    let Ok(rd) = fs::read_dir(wiki_root) else {
        return lines;
    };
    let mut tops: Vec<String> = rd
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.') && !skip.contains(n.as_str()))
        .collect();
    tops.sort();
    for top in tops {
        let base = wiki_root.join(&top);
        if let Ok(paths) = collect_md_paths_budgeted(&base, 2000) {
            for p in paths {
                if let Some(line) = wikilink_path_for_wiki_root(wiki_root, &p) {
                    lines.push(line);
                }
            }
        }
    }
    lines.sort();
    lines.dedup();
    lines
}

/// `root` is the `debug/` directory (parent of `wiki/`).
fn rebuild_index(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let wiki_root = root.join("wiki");
    let concept_dir = wiki_root.join("concepts");
    let summary_dir = wiki_root.join("summaries");
    let mut concepts = Vec::new();
    let mut summaries = Vec::new();

    if concept_dir.exists() {
        for e in fs::read_dir(&concept_dir)? {
            let p = e?.path();
            if p.extension().and_then(|x| x.to_str()) == Some("md")
                && let Some(stem) = p.file_stem().and_then(|x| x.to_str())
            {
                concepts.push(format!("- [[{}]]", stem));
            }
        }
    }
    if summary_dir.exists() {
        for e in fs::read_dir(&summary_dir)? {
            let p = e?.path();
            if p.extension().and_then(|x| x.to_str()) == Some("md")
                && let Some(stem) = p.file_stem().and_then(|x| x.to_str())
            {
                summaries.push(format!("- [[{}]]", stem));
            }
        }
    }
    concepts.sort();
    summaries.sort();
    if concepts.is_empty() {
        concepts.push("- (none yet)".to_string());
    }
    if summaries.is_empty() {
        summaries.push("- (none yet)".to_string());
    }
    let bundled = list_bundled_reference_lines(&wiki_root);
    let bundled_block = if bundled.is_empty() {
        String::new()
    } else {
        format!(
            "\n## Bundled reference markdown\n\n{}\n",
            bundled.join("\n")
        )
    };
    let idx = format!(
        "# Knowledge Compiler Index\n\n## Concepts\n{}\n\n## Source Summaries\n{}{}",
        concepts.join("\n"),
        summaries.join("\n"),
        bundled_block
    );
    fs::write(wiki_root.join("index.md"), idx)?;
    Ok(())
}

fn extract_wikilinks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 3 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let start = i + 2;
            let mut j = start;
            while j + 1 < bytes.len() && !(bytes[j] == b']' && bytes[j + 1] == b']') {
                j += 1;
            }
            if j + 1 < bytes.len() && j > start {
                let raw = text[start..j].trim();
                if !raw.is_empty() {
                    out.push(raw.to_string());
                }
                i = j + 2;
                continue;
            }
            break;
        }
        i += 1;
    }
    out
}

fn ensure_concept_source_backlink(
    concept_path: &Path,
    concept_title: &str,
    summary_stem: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut body = read_or_empty(concept_path);
    let source_link = format!("[[{}]]", summary_stem);
    if body.contains(&source_link) {
        return Ok(());
    }
    if body.trim().is_empty() {
        body = format!(
            "# {}\n\n## Summary\nStub concept generated from summary links.\n\n## Sources\n- {}\n",
            concept_title, source_link
        );
    } else if body.contains("## Sources") {
        body.push_str(&format!("\n- {}\n", source_link));
    } else {
        body.push_str(&format!("\n\n## Sources\n- {}\n", source_link));
    }
    fs::write(concept_path, body)?;
    Ok(())
}

fn ensure_concept_related_backlink(
    concept_path: &Path,
    concept_title: &str,
    from_concept_stem: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut body = read_or_empty(concept_path);
    let back = format!("[[{}]]", from_concept_stem);
    if body.contains(&back) {
        return Ok(());
    }
    if body.trim().is_empty() {
        body = format!(
            "# {}\n\n## Summary\nStub concept generated from wikilinks.\n\n## Related Concepts\n- {}\n",
            concept_title, back
        );
    } else if body.contains("## Related Concepts") {
        body.push_str(&format!("\n- {}\n", back));
    } else {
        body.push_str(&format!("\n\n## Related Concepts\n- {}\n", back));
    }
    fs::write(concept_path, body)?;
    Ok(())
}

/// `root` is the `debug/` directory (parent of `wiki/`).
fn normalize_and_repair_wiki(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let concept_dir = root.join("wiki/concepts");
    let summary_dir = root.join("wiki/summaries");
    fs::create_dir_all(&concept_dir)?;
    fs::create_dir_all(&summary_dir)?;

    // 1) Normalize concept filenames to slug format.
    let mut concept_files = Vec::new();
    for e in fs::read_dir(&concept_dir)? {
        let p = e?.path();
        if p.extension().and_then(|x| x.to_str()) == Some("md") {
            concept_files.push(p);
        }
    }
    for p in concept_files {
        let stem = p.file_stem().and_then(|x| x.to_str()).unwrap_or("");
        let normalized = slugify(stem);
        if normalized.is_empty() || normalized == stem {
            continue;
        }
        let target = concept_dir.join(format!("{normalized}.md"));
        if target.exists() {
            let src = read_or_empty(&p);
            let mut dst = read_or_empty(&target);
            if !src.trim().is_empty() {
                dst.push_str("\n\n");
                dst.push_str(&src);
                fs::write(&target, dst)?;
            }
            fs::remove_file(&p)?;
        } else {
            fs::rename(&p, &target)?;
        }
    }

    // 2) Ensure concept pages exist for summary wikilinks and include backlinks.
    for e in fs::read_dir(&summary_dir)? {
        let p = e?.path();
        if p.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let summary_stem = p.file_stem().and_then(|x| x.to_str()).unwrap_or("summary");
        let summary_text = read_or_empty(&p);
        let links = extract_wikilinks(&summary_text);
        for link in links {
            let slug = slugify(&link);
            if slug.is_empty() || slug == "index" {
                continue;
            }
            let concept_path = concept_dir.join(format!("{slug}.md"));
            ensure_concept_source_backlink(&concept_path, &link, summary_stem)?;
        }
    }

    // 3) Concept-to-concept wikilinks: stubs + reciprocal Related Concepts backlinks.
    for e in fs::read_dir(&concept_dir)? {
        let p = e?.path();
        if p.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let from_stem = p.file_stem().and_then(|x| x.to_str()).unwrap_or("concept");
        if from_stem == "index" {
            continue;
        }
        let text = read_or_empty(&p);
        let links = extract_wikilinks(&text);
        for link in links {
            let slug = slugify(&link);
            if slug.is_empty() || slug == "index" || slug == from_stem {
                continue;
            }
            let target_path = concept_dir.join(format!("{slug}.md"));
            ensure_concept_related_backlink(&target_path, &link, from_stem)?;
        }
    }

    rebuild_index(root)?;
    Ok(())
}

fn run_with_progress<F>(
    path: Option<&str>,
    source: &str,
    question: Option<&str>,
    api_url: Option<&str>,
    mut on_step: F,
) -> Result<RunArtifacts, Box<dyn std::error::Error>>
where
    F: FnMut(&str, &str, &Path),
{
    validate(path)?;
    let root = app_root(path);
    let work = local_agent_app_workdir(&root);
    let (main, agents) = load_spec(&root)?;
    let api = api_url.unwrap_or("http://127.0.0.1:3456");
    ensure_dirs(&work)?;
    let q = question
        .map(ToString::to_string)
        .or(main.meta.default_question.clone())
        .unwrap_or_else(|| "What changed?".to_string());

    let mut latest_source = ingest_assets_markdown(&work_debug(&work), source)?;
    let run_stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let log_file = work_debug(&work)
        .join("scratch")
        .join(format!("orchestration-{run_stamp}.md"));
    let mut log = String::new();
    let mut task_outputs: Vec<(String, PathBuf)> = Vec::new();
    let mut artifacts = RunArtifacts::default();
    let total_steps = main.meta.pipeline.len();
    let run_title = main
        .meta
        .display_name
        .as_deref()
        .unwrap_or(&main.meta.id);
    log.push_str("# Agent App Run\n\n");
    log.push_str(&format!(
        "- Horde: {} ({})\n- Source: {}\n- Question: {}\n\n",
        run_title, main.meta.id, source, q
    ));
    println!("Starting agent app run: {} ({})", run_title, main.meta.id);
    println!("Task count: {}", total_steps);

    for (idx, step) in main.meta.pipeline.iter().enumerate() {
        let agent = agents
            .get(step)
            .ok_or_else(|| format!("missing step agent: {step}"))?;
        println!(
            "[{}/{}] {} ({})",
            idx + 1,
            total_steps,
            step,
            agent.meta.kind
        );
        log.push_str(&format!("## Step: {} ({})\n\n", step, agent.meta.kind));
        match agent.meta.kind.as_str() {
            "ingest" => {
                latest_source = ingest_assets_markdown(&work_debug(&work), source)?;
                log.push_str(&format!("- output: {}\n\n", latest_source.display()));
                task_outputs.push((step.clone(), latest_source.clone()));
                on_step(step, agent.meta.kind.as_str(), latest_source.as_path());
                println!("  -> {}", latest_source.display());
            }
            "compile" => {
                let prompt_path = root.join(
                    agent
                        .meta
                        .prompt_file
                        .as_deref()
                        .unwrap_or("prompts/compiler.md"),
                );
                let prompt = read_or_empty(&prompt_path);
                let summary_out = work.join(
                    agent
                        .meta
                        .output
                        .as_deref()
                        .unwrap_or("debug/wiki/summaries/latest.md"),
                );
                let src = read_or_empty(&latest_source);
                let msg = format!(
                    "{prompt}\n\nSource file: {}\n\n{src}\n",
                    latest_source.display()
                );
                let reply = chat_no_tools(api, &msg)?;
                let reply = normalize_markdown_sections(
                    &reply,
                    "Source Summary",
                    &["Summary", "Extracted Concepts", "Notable Claims", "Sources"],
                    "Fallback summary due to empty or malformed model output.",
                );
                fs::write(&summary_out, &reply)?;
                normalize_and_repair_wiki(&work_debug(&work))?;
                log.push_str(&format!("- output: {}\n\n", summary_out.display()));
                task_outputs.push((step.clone(), summary_out.clone()));
                artifacts.summary = Some(summary_out.clone());
                on_step(step, agent.meta.kind.as_str(), summary_out.as_path());
                println!("  -> {}", summary_out.display());
            }
            "ask" => {
                let prompt_path = root.join(
                    agent
                        .meta
                        .prompt_file
                        .as_deref()
                        .unwrap_or("prompts/query.md"),
                );
                let prompt = read_or_empty(&prompt_path);
                let out = work.join(
                    agent
                        .meta
                        .output
                        .as_deref()
                        .unwrap_or("debug/derived/reports/latest.md"),
                );
                let idx = read_or_empty(&work_debug(&work).join("wiki/index.md"));
                let msg = format!("{prompt}\n\nQuestion: {q}\n\nWiki index:\n{idx}\n");
                let reply = chat_no_tools(api, &msg)?;
                let reply = normalize_markdown_sections(
                    &reply,
                    "Knowledge Compiler Answer",
                    &["Question", "Response", "Sources Used"],
                    "Fallback answer due to empty or malformed model output.",
                );
                fs::write(&out, &reply)?;
                log.push_str(&format!("- output: {}\n\n", out.display()));
                task_outputs.push((step.clone(), out.clone()));
                artifacts.report = Some(out.clone());
                on_step(step, agent.meta.kind.as_str(), out.as_path());
                println!("  -> {}", out.display());
            }
            "lint" => {
                let prompt_path = root.join(
                    agent
                        .meta
                        .prompt_file
                        .as_deref()
                        .unwrap_or("prompts/lint.md"),
                );
                let prompt = read_or_empty(&prompt_path);
                let out = work.join(
                    agent
                        .meta
                        .output
                        .as_deref()
                        .unwrap_or("debug/derived/lint/latest.md"),
                );
                let idx = read_or_empty(&work_debug(&work).join("wiki/index.md"));
                let msg = format!("{prompt}\n\nWiki index:\n{idx}\n");
                let reply = chat_no_tools(api, &msg)?;
                let reply = normalize_markdown_sections(
                    &reply,
                    "Knowledge Lint Report",
                    &[
                        "Snapshot",
                        "Issues",
                        "Suggested Fixes",
                        "Candidate New Articles",
                    ],
                    "- Fallback lint output due to empty or malformed model output.",
                );
                fs::write(&out, &reply)?;
                let paste = write_paste_me_file(&work, Some(source))?;
                log.push_str(&format!(
                    "- output: {}\n- obsidian paste: {}\n\n",
                    out.display(),
                    paste.display()
                ));
                task_outputs.push((step.clone(), out.clone()));
                artifacts.lint = Some(out.clone());
                on_step(step, agent.meta.kind.as_str(), paste.as_path());
                println!("  -> {}", out.display());
                println!("  -> {}", paste.display());
            }
            other => return Err(format!("unsupported agent kind: {other}").into()),
        }
    }

    fs::write(&log_file, log)?;
    println!("\nSub-agent execution trace:");
    for (task, out) in &task_outputs {
        println!("- {} -> {}", task, out.display());
    }

    let latest_summary = latest_md_in(&work_debug(&work).join("wiki").join("summaries"));
    let latest_report = latest_md_in(&work_debug(&work).join("derived").join("reports"));
    let latest_lint = latest_md_in(&work_debug(&work).join("derived").join("lint"));

    println!("\nFinal output artifacts:");
    if let Some(p) = latest_summary {
        println!("- summary: {}", p.display());
    }
    if let Some(p) = latest_report {
        println!("- report: {}", p.display());
    }
    if let Some(p) = latest_lint {
        println!("- lint: {}", p.display());
    }
    println!("Agent app run complete. Log: {}", log_file.display());
    artifacts.log = Some(log_file);
    Ok(artifacts)
}

pub fn run(
    path: Option<&str>,
    source: &str,
    question: Option<&str>,
    api_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = run_with_progress(path, source, question, api_url, |_step, _kind, _output| {})?;
    Ok(())
}

fn post_json(
    api: &str,
    route: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest_blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let resp = client
        .post(format!("{}{}", api.trim_end_matches('/'), route))
        .json(&payload)
        .send()
        .map_err(|e| {
            friendly_http_error(
                api,
                route,
                &format!("{}{}", api.trim_end_matches('/'), route),
                &e,
            )
        })?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().unwrap_or_else(|_| serde_json::json!({}));
    if !status.is_success() {
        return Err(friendly_http_status_error(
            api,
            route,
            &format!("{}{}", api.trim_end_matches('/'), route),
            status.as_u16(),
            Some(v),
        )
        .into());
    }
    Ok(v)
}

fn friendly_http_error(api: &str, route: &str, url: &str, err: &reqwest::Error) -> String {
    let mut msg = format!("request failed for {} ({}): {}", route, url, err);
    msg.push_str("\nPossible root causes:");
    if err.is_connect() {
        msg.push_str("\n- Kowalski server is not running or not reachable.");
        msg.push_str("\n- API URL is wrong.");
    } else if err.is_timeout() {
        msg.push_str("\n- Server is running but timed out.");
        msg.push_str("\n- LLM/provider backend is slow or blocked.");
    } else {
        msg.push_str("\n- Network or server-side error.");
    }
    msg.push_str("\nHow to fix:");
    msg.push_str("\n- Start server: cargo run -p kowalski --bin kowalski");
    msg.push_str(&format!(
        "\n- Verify health: curl {}/api/health",
        api.trim_end_matches('/')
    ));
    msg.push_str(&format!(
        "\n- If using custom API, set KOWALSKI_API and retry (current: {}).",
        api
    ));
    msg
}

fn friendly_http_status_error(
    api: &str,
    route: &str,
    url: &str,
    status_code: u16,
    body: Option<serde_json::Value>,
) -> String {
    let mut msg = format!(
        "request failed for {} ({}): HTTP {}",
        route, url, status_code
    );
    if let Some(v) = body
        && v != serde_json::json!({})
    {
        msg.push_str(&format!("\nResponse body: {}", v));
    }
    msg.push_str("\nPossible root causes:");
    if status_code == 404 {
        msg.push_str("\n- Endpoint is missing (version mismatch or wrong API URL).");
    } else if status_code >= 500 {
        msg.push_str("\n- Kowalski server internal error.");
        msg.push_str("\n- Upstream LLM/provider failure.");
    } else if status_code == 401 || status_code == 403 {
        msg.push_str("\n- Authentication/authorization problem.");
    } else {
        msg.push_str("\n- Request rejected by server configuration.");
    }
    msg.push_str("\nHow to fix:");
    msg.push_str("\n- Ensure server is running: cargo run -p kowalski --bin kowalski");
    msg.push_str(&format!(
        "\n- Verify health: curl {}/api/health",
        api.trim_end_matches('/')
    ));
    msg.push_str(&format!(
        "\n- Confirm API URL and route availability (current API: {}, route: {}).",
        api, route
    ));
    msg
}

fn latest_md_in(dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    files.sort();
    files.pop()
}

pub fn federate_delegate(
    api_url: Option<&str>,
    capability: &str,
    source: &str,
    question: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api = api_url.unwrap_or("http://127.0.0.1:3456");
    let task_id = format!("kc-{}", Utc::now().timestamp());
    let instruction = format!(
        "kc.run:{}|{}",
        source,
        question.unwrap_or("What changed in the latest source?")
    );
    let body = serde_json::json!({
        "task_id": task_id,
        "instruction": instruction,
        "capability": capability,
    });
    let out = post_json(api, "/api/federation/delegate", body)?;
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

pub fn federate_worker(
    path: Option<&str>,
    api_url: Option<&str>,
    agent_id: &str,
    topic: Option<&str>,
    role: Option<&str>,
    capability_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api = api_url.unwrap_or("http://127.0.0.1:3456");
    let root = app_root(path);
    let topic = topic.unwrap_or("federation");

    let role = role.map(|s| s.to_string());
    let capabilities: Vec<String> = if let Some(cap) = capability_override {
        vec![cap.to_string()]
    } else if let Some(r) = role.as_deref() {
        vec![format!("kc.{}", r)]
    } else {
        vec!["knowledge-compiler".to_string(), "kc.run".to_string()]
    };

    let reg = serde_json::json!({
        "id": agent_id,
        "capabilities": capabilities.clone(),
    });
    let _ = post_json(api, "/api/federation/register", reg)?;
    println!(
        "Registered worker `{}` (role={}, capabilities={}). Listening on topic `{}`.",
        agent_id,
        role.as_deref().unwrap_or("(legacy: kc.run)"),
        capabilities.join(","),
        topic
    );

    // SSE worker must keep the HTTP connection open indefinitely.
    // A zero-duration timeout causes immediate failure in reqwest.
    let client = reqwest_blocking::Client::builder().build()?;
    let stream_url = format!(
        "{}/api/federation/stream?topic={}",
        api.trim_end_matches('/'),
        topic
    );
    let resp = client.get(stream_url).send()?;
    if !resp.status().is_success() {
        return Err(format!("stream failed: HTTP {}", resp.status()).into());
    }
    let reader = std::io::BufReader::new(resp);

    for line in reader.lines() {
        let line = match line {
            Ok(v) => v,
            Err(e) => {
                eprintln!("federation stream decode warning (ignored): {}", e);
                continue;
            }
        };
        if !line.starts_with("data: ") {
            continue;
        }
        let data = &line[6..];
        let env: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let payload = env
            .get("payload")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        if payload.get("kind").and_then(|x| x.as_str()) != Some("task_delegate") {
            continue;
        }
        if payload.get("to_agent").and_then(|x| x.as_str()) != Some(agent_id) {
            continue;
        }
        let task_id = payload
            .get("task_id")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown-task")
            .to_string();
        let instruction = payload
            .get("instruction")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(role_kind) = role.as_deref() {
            handle_role_delegate(
                api,
                topic,
                agent_id,
                role_kind,
                &root,
                &task_id,
                &instruction,
            );
        } else {
            handle_legacy_run_delegate(api, topic, agent_id, &root, &task_id, &instruction);
        }
        let _ = post_json(
            api,
            "/api/federation/heartbeat",
            serde_json::json!({ "agent_id": agent_id }),
        );
    }
    Ok(())
}

fn parse_horde_instruction(instruction: &str) -> Option<HordeInstruction> {
    let v: serde_json::Value = serde_json::from_str(instruction).ok()?;
    let horde = v
        .get("horde")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let run_id = v
        .get("run_id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let step = v
        .get("step")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let kind = v
        .get("kind")
        .and_then(|x| x.as_str())
        .unwrap_or(&step)
        .to_string();
    if horde.is_empty() || run_id.is_empty() || step.is_empty() {
        return None;
    }
    Some(HordeInstruction {
        horde,
        run_id,
        step,
        kind,
        source: v
            .get("source")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
        question: v
            .get("question")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
        previous_artifact: v
            .get("previous_artifact")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
        horde_root: v
            .get("horde_root")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
        workdir: v
            .get("workdir")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
    })
}

#[derive(Debug, Clone)]
struct HordeInstruction {
    horde: String,
    run_id: String,
    step: String,
    kind: String,
    source: Option<String>,
    question: Option<String>,
    previous_artifact: Option<String>,
    horde_root: Option<String>,
    workdir: Option<String>,
}

fn publish_acl(api: &str, topic: &str, sender: &str, payload: serde_json::Value) {
    let body = serde_json::json!({
        "sender": sender,
        "topic": topic,
        "payload": payload,
    });
    if let Err(e) = post_json(api, "/api/federation/publish", body) {
        eprintln!("publish failed: {}", e);
    }
}

fn publish_task_started(
    api: &str,
    topic: &str,
    sender: &str,
    instr: &HordeInstruction,
    text: &str,
) {
    publish_acl(
        api,
        topic,
        sender,
        serde_json::json!({
            "kind": "task_started",
            "run_id": instr.run_id,
            "horde": instr.horde,
            "step": instr.step,
            "agent": sender,
            "text": text,
        }),
    );
}

fn publish_agent_message(
    api: &str,
    topic: &str,
    sender: &str,
    instr: &HordeInstruction,
    text: &str,
) {
    publish_acl(
        api,
        topic,
        sender,
        serde_json::json!({
            "kind": "agent_message",
            "run_id": instr.run_id,
            "horde": instr.horde,
            "from": sender,
            "step": instr.step,
            "text": text,
        }),
    );
}

fn publish_task_finished(
    api: &str,
    topic: &str,
    sender: &str,
    instr: &HordeInstruction,
    success: bool,
    artifact: Option<&str>,
    summary: &str,
) {
    publish_acl(
        api,
        topic,
        sender,
        serde_json::json!({
            "kind": "task_finished",
            "run_id": instr.run_id,
            "horde": instr.horde,
            "step": instr.step,
            "agent": sender,
            "success": success,
            "artifact": artifact,
            "summary": summary,
        }),
    );
}

fn publish_task_result(
    api: &str,
    topic: &str,
    sender: &str,
    task_id: &str,
    outcome: &str,
    success: bool,
) {
    publish_acl(
        api,
        topic,
        sender,
        serde_json::json!({
            "kind": "task_result",
            "task_id": task_id,
            "from_agent": sender,
            "outcome": outcome,
            "success": success,
        }),
    );
}

fn handle_role_delegate(
    api: &str,
    topic: &str,
    agent_id: &str,
    role_kind: &str,
    fallback_root: &Path,
    task_id: &str,
    instruction: &str,
) {
    let Some(instr) = parse_horde_instruction(instruction) else {
        let summary = format!(
            "rejected: instruction is not a horde JSON envelope (task_id={})",
            task_id
        );
        eprintln!("{}", summary);
        publish_task_result(api, topic, agent_id, task_id, &summary, false);
        return;
    };

    if instr.kind != role_kind {
        let summary = format!(
            "rejected: agent role `{}` does not match instruction kind `{}`",
            role_kind, instr.kind
        );
        publish_task_finished(api, topic, agent_id, &instr, false, None, &summary);
        publish_task_result(api, topic, agent_id, task_id, &summary, false);
        return;
    }

    let workspace_root = instr
        .horde_root
        .clone()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .unwrap_or_else(|| fallback_root.to_path_buf());
    let workdir = instr
        .workdir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.clone());

    publish_task_started(
        api,
        topic,
        agent_id,
        &instr,
        &format!("{} agent starting", role_kind),
    );

    let result = match role_kind {
        "ingest" => execute_ingest(api, topic, agent_id, &instr, &workspace_root, &workdir),
        "compile" => execute_compile(api, topic, agent_id, &instr, &workspace_root, &workdir),
        "ask" => execute_ask(api, topic, agent_id, &instr, &workspace_root, &workdir),
        "lint" => execute_lint(api, topic, agent_id, &instr, &workspace_root, &workdir),
        other => Err(format!("unsupported role kind `{}`", other).into()),
    };

    match result {
        Ok((artifact, summary)) => {
            publish_task_finished(
                api,
                topic,
                agent_id,
                &instr,
                true,
                Some(&artifact),
                &summary,
            );
            publish_task_result(
                api,
                topic,
                agent_id,
                task_id,
                &format!("artifact={}; {}", artifact, summary),
                true,
            );
        }
        Err(e) => {
            let summary = format!("{} step failed: {}", role_kind, e);
            publish_task_finished(api, topic, agent_id, &instr, false, None, &summary);
            publish_task_result(api, topic, agent_id, task_id, &summary, false);
        }
    }
}

fn execute_ingest(
    api: &str,
    topic: &str,
    agent_id: &str,
    instr: &HordeInstruction,
    _workspace_root: &Path,
    workdir: &Path,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let _ = api;
    let _ = topic;
    let _ = agent_id;
    let source = instr
        .source
        .as_deref()
        .ok_or("ingest: missing `source` in horde instruction")?;
    ensure_dirs(workdir)?;
    let source_list = parse_input_assets(source);
    let path = ingest_assets_markdown(&work_debug(workdir), source)?;
    Ok((
        path.display().to_string(),
        format!(
            "Captured {} source(s) into raw collection: {}",
            source_list.len(),
            path.display()
        ),
    ))
}

fn execute_compile(
    api: &str,
    topic: &str,
    agent_id: &str,
    instr: &HordeInstruction,
    workspace_root: &Path,
    workdir: &Path,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    ensure_dirs(workdir)?;
    let prompt_path = workspace_root.join("prompts/compiler.md");
    let prompt = read_or_empty(&prompt_path);
    let summary_out = work_debug(workdir).join("wiki/summaries/latest.md");
    let source_path = instr
        .previous_artifact
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| latest_md_in(&work_debug(workdir).join("raw/sources")))
        .ok_or("compile: no input artifact available (run ingest first)")?;
    let src = read_or_empty(&source_path);
    publish_agent_message(
        api,
        topic,
        agent_id,
        instr,
        &format!("Reading raw source: {}", source_path.display()),
    );
    let msg = format!(
        "{prompt}\n\nSource file: {}\n\n{src}\n",
        source_path.display()
    );
    let reply = chat_no_tools(api, &msg)?;
    let reply = normalize_markdown_sections(
        &reply,
        "Source Summary",
        &["Summary", "Extracted Concepts", "Notable Claims", "Sources"],
        "Fallback summary due to empty or malformed model output.",
    );
    fs::write(&summary_out, &reply)?;
    normalize_and_repair_wiki(&work_debug(workdir))?;
    Ok((
        summary_out.display().to_string(),
        format!("Compiled wiki summary at {}", summary_out.display()),
    ))
}

fn execute_ask(
    api: &str,
    topic: &str,
    agent_id: &str,
    instr: &HordeInstruction,
    workspace_root: &Path,
    workdir: &Path,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    ensure_dirs(workdir)?;
    let prompt_path = workspace_root.join("prompts/query.md");
    let prompt = read_or_empty(&prompt_path);
    let out = work_debug(workdir).join("derived/reports/latest.md");
    let idx = read_or_empty(&work_debug(workdir).join("wiki/index.md"));
    let q = instr
        .question
        .clone()
        .unwrap_or_else(|| "What changed in the latest source?".to_string());
    publish_agent_message(
        api,
        topic,
        agent_id,
        instr,
        &format!("Answering question: {}", q),
    );
    let msg = format!("{prompt}\n\nQuestion: {q}\n\nWiki index:\n{idx}\n");
    let reply = chat_no_tools(api, &msg)?;
    let reply = normalize_markdown_sections(
        &reply,
        "Knowledge Compiler Answer",
        &["Question", "Response", "Sources Used"],
        "Fallback answer due to empty or malformed model output.",
    );
    fs::write(&out, &reply)?;
    Ok((
        out.display().to_string(),
        format!("Answer report written to {}", out.display()),
    ))
}

fn execute_lint(
    api: &str,
    topic: &str,
    agent_id: &str,
    instr: &HordeInstruction,
    workspace_root: &Path,
    workdir: &Path,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    ensure_dirs(workdir)?;
    let prompt_path = workspace_root.join("prompts/lint.md");
    let prompt = read_or_empty(&prompt_path);
    let out = work_debug(workdir).join("derived/lint/latest.md");
    let idx = read_or_empty(&work_debug(workdir).join("wiki/index.md"));
    publish_agent_message(api, topic, agent_id, instr, "Linting wiki index");
    let msg = format!("{prompt}\n\nWiki index:\n{idx}\n");
    let reply = chat_no_tools(api, &msg)?;
    let reply = normalize_markdown_sections(
        &reply,
        "Knowledge Lint Report",
        &[
            "Snapshot",
            "Issues",
            "Suggested Fixes",
            "Candidate New Articles",
        ],
        "- Fallback lint output due to empty or malformed model output.",
    );
    fs::write(&out, &reply)?;
    let paste = write_paste_me_file(workdir, instr.source.as_deref())?;
    Ok((
        paste.display().to_string(),
        format!(
            "Obsidian paste pack at {} (lint: {})",
            paste.display(),
            out.display()
        ),
    ))
}

fn handle_legacy_run_delegate(
    api: &str,
    topic: &str,
    agent_id: &str,
    root: &Path,
    task_id: &str,
    instruction: &str,
) {
    let mut success = true;
    let outcome: String;
    if let Some(raw) = instruction.strip_prefix("kc.run:") {
        let mut parts = raw.splitn(2, '|');
        let source = parts.next().unwrap_or("").trim();
        let question = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("What changed in the latest source?");
        if source.is_empty() {
            success = false;
            outcome = "missing source in instruction".to_string();
        } else {
            let root_s = root.to_string_lossy().to_string();
            let work = local_agent_app_workdir(root);
            let run_out = run_with_progress(
                Some(root_s.as_str()),
                source,
                Some(question),
                Some(api),
                |_step, _kind, _output| {},
            );
            match run_out {
                Err(e) => {
                    success = false;
                    outcome = format!("run failed: {}", e);
                }
                Ok(done) => {
                    let report = done
                        .report
                        .or_else(|| latest_md_in(&work_debug(&work).join("derived/reports")))
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    let lint_disp = done
                        .lint
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    let summary = done
                        .summary
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    let log = done
                        .log
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    outcome = format!(
                        "run complete; summary={}; report={}; lint={}; log={}",
                        summary, report, lint_disp, log
                    );
                }
            }
        }
    } else {
        success = false;
        outcome = format!("unsupported instruction: {}", instruction);
    }
    publish_task_result(api, topic, agent_id, task_id, &outcome, success);
}

pub fn proof_check(
    path: Option<&str>,
    api_url: Option<&str>,
    agent_id: Option<&str>,
    capability: Option<&str>,
    source: Option<&str>,
    question: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    let work = local_agent_app_workdir(&root);
    let api = api_url.unwrap_or("http://127.0.0.1:3456");
    let agent_id = agent_id.unwrap_or("kc-worker-1");
    let capability = capability.unwrap_or("kc.run");
    let source = source.unwrap_or("https://example.com/article");
    let question = question.unwrap_or("What changed?");

    // Preflight checks
    println!("Preflight:");
    validate(Some(root.to_string_lossy().as_ref()))?;
    let health = reqwest_blocking::get(format!("{}/api/health", api.trim_end_matches('/')));
    match health {
        Ok(r) if r.status().is_success() => println!("- API reachable at {}", api),
        Ok(r) => println!("- API responded with HTTP {} at {}", r.status(), api),
        Err(e) => println!("- API not reachable at {} ({})", api, e),
    }
    println!("- App path: {}", root.display());

    println!("\nProof-run checklist (3 terminals):");
    println!("1) Terminal A: start server");
    println!("   cargo run -p kowalski --bin kowalski");
    println!("2) Terminal B: start worker");
    println!(
        "   cargo run -p kowalski-cli -- agent-app worker --path \"{}\" --api \"{}\" \"{}\"",
        root.display(),
        api,
        agent_id
    );
    println!("3) Terminal C: delegate task");
    println!(
        "   cargo run -p kowalski-cli -- agent-app delegate --api \"{}\" --question \"{}\" \"{}\" \"{}\"",
        api, question, capability, source
    );
    println!("\nVerify artifacts (under {}):", work.display());
    println!(
        "- latest report: {}",
        latest_md_in(&work_debug(&work).join("derived/reports"))
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none yet)".to_string())
    );
    println!(
        "- lint report: {}",
        work_debug(&work).join("derived/lint/latest.md").display()
    );
    println!(
        "- latest run log: {}",
        latest_md_in(&work_debug(&work).join("scratch"))
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none yet)".to_string())
    );
    println!(
        "- paste pack: {}",
        work.join(PASTE_ME_FILENAME).display()
    );
    Ok(())
}
