//! Markdown-defined app agent orchestration (`main-agent.md` + `agents/*.md`).

use chrono::Utc;
use reqwest::blocking as reqwest_blocking;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct MainAgentMeta {
    name: String,
    available_agents: Vec<String>,
    pipeline: Vec<String>,
    default_question: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubAgentMeta {
    name: String,
    kind: String,
    prompt_file: Option<String>,
    output: Option<String>,
}

#[derive(Debug)]
struct AgentDoc<T> {
    meta: T,
    body: String,
    path: PathBuf,
}

fn parse_markdown_with_toml_frontmatter<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<AgentDoc<T>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let mut lines = raw.lines();
    if lines.next().map(|s| s.trim()) != Some("---") {
        return Err(format!("Missing frontmatter start in {}", path.display()).into());
    }
    let mut fm = String::new();
    let mut body = String::new();
    let mut in_fm = true;
    for line in raw.lines().skip(1) {
        if in_fm && line.trim() == "---" {
            in_fm = false;
            continue;
        }
        if in_fm {
            fm.push_str(line);
            fm.push('\n');
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    if in_fm {
        return Err(format!("Missing frontmatter end in {}", path.display()).into());
    }
    let meta: T = toml::from_str(&fm)?;
    Ok(AgentDoc {
        meta,
        body: body.trim().to_string(),
        path: path.to_path_buf(),
    })
}

fn app_root(path: Option<&str>) -> PathBuf {
    path.map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/knowledge-compiler"))
}

fn main_agent_path(root: &Path) -> PathBuf {
    root.join("main-agent.md")
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
) -> Result<(AgentDoc<MainAgentMeta>, BTreeMap<String, AgentDoc<SubAgentMeta>>), Box<dyn std::error::Error>>
{
    let main = parse_markdown_with_toml_frontmatter::<MainAgentMeta>(&main_agent_path(root))?;
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

pub fn list_agents(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    let (main, agents) = load_spec(&root)?;
    println!("Main agent: {}", main.meta.name);
    println!("Pipeline: {}", main.meta.pipeline.join(" -> "));
    println!("Available agents:");
    for name in main.meta.available_agents {
        if let Some(agent) = agents.get(&name) {
            println!("- {} ({})", name, agent.meta.kind);
        } else {
            println!("- {} (missing definition)", name);
        }
    }
    Ok(())
}

pub fn validate(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    let (main, agents) = load_spec(&root)?;
    let mut errs = Vec::new();
    let defs: BTreeSet<_> = agents.keys().cloned().collect();
    let available: BTreeSet<_> = main.meta.available_agents.iter().cloned().collect();

    for name in &main.meta.available_agents {
        if !defs.contains(name) {
            errs.push(format!("available_agents includes missing agent `{name}`"));
        }
    }
    for name in &main.meta.pipeline {
        if !available.contains(name) {
            errs.push(format!("pipeline references undeclared agent `{name}`"));
        }
        if !defs.contains(name) {
            errs.push(format!("pipeline references missing agent definition `{name}`"));
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
        println!("OK - agent app definition is valid");
        return Ok(());
    }
    for e in errs {
        eprintln!("ERROR: {}", e);
    }
    Err("agent app definition invalid".into())
}

fn chat_no_tools(api: &str, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest_blocking::Client::builder().timeout(std::time::Duration::from_secs(120)).build()?;
    let resp = client
        .post(format!("{}/api/chat", api.trim_end_matches('/')))
        .json(&serde_json::json!({
            "message": prompt,
            "use_memory": false,
            "use_tools": false
        }))
        .send()?;
    if !resp.status().is_success() {
        return Err(format!("chat failed: HTTP {}", resp.status()).into());
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
    for rel in [
        "raw/sources",
        "raw/images",
        "wiki/concepts",
        "wiki/summaries",
        "derived/reports",
        "derived/lint",
        "scratch",
    ] {
        fs::create_dir_all(root.join(rel))?;
    }
    Ok(())
}

fn ingest_source(root: &Path, source: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    ensure_dirs(root)?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let out = root.join("raw/sources").join(format!("{stamp}-{}.md", slugify(source)));
    let now = Utc::now().to_rfc3339();
    let content = if source.starts_with("http://") || source.starts_with("https://") {
        match reqwest_blocking::get(source) {
            Ok(resp) => {
                let text = resp.text().unwrap_or_else(|_| "(unable to decode body)".to_string());
                format!(
                    "# Raw Source\n\n- Input: {source}\n- Ingested At: {now}\n\n## Content\n{}\n",
                    text.chars().take(24000).collect::<String>()
                )
            }
            Err(e) => format!(
                "# Raw Source\n\n- Input: {source}\n- Ingested At: {now}\n\n## Fetch Error\n{e}\n"
            ),
        }
    } else {
        format!(
            "# Raw Source\n\n- Input: {source}\n- Ingested At: {now}\n\n## Notes\nText input captured. Prefer full URL for web ingest.\n"
        )
    };
    fs::write(&out, content)?;
    Ok(out)
}

pub fn run(
    path: Option<&str>,
    source: &str,
    question: Option<&str>,
    api_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    validate(path)?;
    let root = app_root(path);
    let (main, agents) = load_spec(&root)?;
    let api = api_url.unwrap_or("http://127.0.0.1:3456");
    ensure_dirs(&root)?;
    let q = question
        .map(ToString::to_string)
        .or(main.meta.default_question.clone())
        .unwrap_or_else(|| "What changed?".to_string());

    let mut latest_source = ingest_source(&root, source)?;
    let run_stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let log_file = root.join("scratch").join(format!("orchestration-{run_stamp}.md"));
    let mut log = String::new();
    log.push_str("# Agent App Run\n\n");
    log.push_str(&format!("- Main agent: {}\n- Source: {}\n- Question: {}\n\n", main.meta.name, source, q));

    for step in &main.meta.pipeline {
        let agent = agents.get(step).ok_or_else(|| format!("missing step agent: {step}"))?;
        log.push_str(&format!("## Step: {} ({})\n\n", step, agent.meta.kind));
        match agent.meta.kind.as_str() {
            "ingest" => {
                latest_source = ingest_source(&root, source)?;
                log.push_str(&format!("- output: {}\n\n", latest_source.display()));
            }
            "compile" => {
                let prompt_path = root.join(agent.meta.prompt_file.as_deref().unwrap_or("prompts/compiler.md"));
                let prompt = read_or_empty(&prompt_path);
                let summary_out = root.join(agent.meta.output.as_deref().unwrap_or("wiki/summaries/latest.md"));
                let src = read_or_empty(&latest_source);
                let msg = format!("{prompt}\n\nSource file: {}\n\n{}\n", latest_source.display(), src);
                let mut reply = chat_no_tools(api, &msg)?;
                if reply.trim().is_empty() || reply.trim() == "{}" {
                    reply = "# Source Summary\n\n## Summary\nFallback summary due to empty model output.\n".to_string();
                }
                fs::write(&summary_out, &reply)?;
                log.push_str(&format!("- output: {}\n\n", summary_out.display()));
            }
            "ask" => {
                let prompt_path = root.join(agent.meta.prompt_file.as_deref().unwrap_or("prompts/query.md"));
                let prompt = read_or_empty(&prompt_path);
                let out = root.join(agent.meta.output.as_deref().unwrap_or("derived/reports/latest.md"));
                let idx = read_or_empty(&root.join("wiki/index.md"));
                let msg = format!("{prompt}\n\nQuestion: {q}\n\nWiki index:\n{idx}\n");
                let mut reply = chat_no_tools(api, &msg)?;
                if reply.trim().is_empty() || reply.trim() == "{}" {
                    reply = format!("# Knowledge Compiler Answer\n\n## Question\n{q}\n\n## Response\nFallback answer.\n");
                }
                fs::write(&out, &reply)?;
                log.push_str(&format!("- output: {}\n\n", out.display()));
            }
            "lint" => {
                let prompt_path = root.join(agent.meta.prompt_file.as_deref().unwrap_or("prompts/lint.md"));
                let prompt = read_or_empty(&prompt_path);
                let out = root.join(agent.meta.output.as_deref().unwrap_or("derived/lint/latest.md"));
                let idx = read_or_empty(&root.join("wiki/index.md"));
                let msg = format!("{prompt}\n\nWiki index:\n{idx}\n");
                let mut reply = chat_no_tools(api, &msg)?;
                if reply.trim().is_empty() || reply.trim() == "{}" {
                    reply = "# Knowledge Lint Report\n\n## Issues\n- Fallback lint output.\n".to_string();
                }
                fs::write(&out, &reply)?;
                log.push_str(&format!("- output: {}\n\n", out.display()));
            }
            other => return Err(format!("unsupported agent kind: {other}").into()),
        }
    }

    fs::write(&log_file, log)?;
    println!("Agent app run complete. Log: {}", log_file.display());
    Ok(())
}
