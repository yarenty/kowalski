//! Local **markdown-staged app** CLI (`app.md` / `horde.md` + `agents/*.md`).
//! Execution rules live in [`kowalski_core::markdown_pipeline`]; this module wires HTTP chat,
//! federation helpers, and filesystem paths.

use kowalski_core::{
    execution_order, resolve_execution_graph, single_predecessor, validate_horde_tree,
};
use kowalski_core::markdown_pipeline::{
    maybe_normalize_markdown, parse_app_manifest, parse_stage_agent, render_context_attachments,
    resolve_manifest_path, AppManifestMeta, StageAgentMeta,
};
use kowalski_core::source_bundle::{ingest_assets_markdown, parse_input_assets};
use chrono::Utc;
use reqwest::blocking as reqwest_blocking;
use std::collections::BTreeMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// Intermediate artifacts (ingest captures, per-stage LLM outputs) live here — for monitoring / debugging only.
const ARTIFACTS_DEBUG_DIR: &str = "debug";
/// Single operator-facing markdown at the `workdir` (or app run) root.
const PASTE_ME_FILENAME: &str = "PASTE_ME.md";

#[inline]
fn work_debug(workdir: &Path) -> PathBuf {
    workdir.join(ARTIFACTS_DEBUG_DIR)
}

/// Default workdir for local `agent-app run` when `horde.md` uses a relative `workdir` (often `output/`).
const LOCAL_AGENT_APP_WORKDIR: &str = "output";

#[inline]
fn local_agent_app_workdir(app_root: &Path) -> PathBuf {
    app_root.join(LOCAL_AGENT_APP_WORKDIR)
}

#[derive(Default, Clone)]
struct RunArtifacts {
    summary: Option<PathBuf>,
    report: Option<PathBuf>,
    handoff: Option<PathBuf>,
    log: Option<PathBuf>,
}

#[derive(Debug)]
struct AgentDoc<T> {
    meta: T,
    path: PathBuf,
}

type AgentSpecMap = BTreeMap<String, AgentDoc<StageAgentMeta>>;

fn app_root(path: Option<&str>) -> PathBuf {
    path.map(PathBuf::from)
        .or_else(|| std::env::var("KOWALSKI_AGENT_APP_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("examples/knowledge-compiler"))
}

fn agents_dir(root: &Path) -> PathBuf {
    root.join("agents")
}

fn load_spec(
    root: &Path,
) -> Result<(AgentDoc<AppManifestMeta>, AgentSpecMap), Box<dyn std::error::Error>> {
    let mpath = resolve_manifest_path(root);
    if !mpath.is_file() {
        return Err(format!(
            "missing manifest (tried {} and {}), expected next to agents/",
            root.join("app.md").display(),
            root.join("horde.md").display()
        )
        .into());
    }
    let meta = parse_app_manifest(&mpath).map_err(|e| e.to_string())?;
    let main = AgentDoc {
        meta,
        path: mpath,
    };
    let mut map = BTreeMap::new();
    for entry in fs::read_dir(agents_dir(root))? {
        let path = entry?.path();
        if path.extension().and_then(|x| x.to_str()) != Some("md") {
            continue;
        }
        let sm = parse_stage_agent(&path).map_err(|e| e.to_string())?;
        map.insert(
            sm.name.clone(),
            AgentDoc {
                meta: sm,
                path: path.to_path_buf(),
            },
        );
    }
    Ok((main, map))
}

pub fn list_agents(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    let (main, agents) = load_spec(&root)?;
    let title = main
        .meta
        .display_name
        .as_deref()
        .unwrap_or(&main.meta.id);
    println!("App: {} ({})", title, main.meta.id);
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
        if !main.meta.pipeline.contains(name)
            && let Some(agent) = agents.get(name)
        {
            println!(
                "- {} ({}) [not in manifest pipeline — remove or add to pipeline]",
                name, agent.meta.kind
            );
        }
    }
    Ok(())
}

pub fn validate(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let root = app_root(path);
    validate_horde_tree(&root).map_err(|e| e.to_string())?;
    println!("OK - manifest + agents/ definitions are valid");
    Ok(())
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
    let status = resp.status();
    let body_text = resp.text().unwrap_or_default();
    if !status.is_success() {
        let detail = body_text.trim();
        let body_json = if detail.is_empty() {
            None
        } else {
            serde_json::from_str::<serde_json::Value>(detail)
                .ok()
                .or_else(|| Some(serde_json::json!({ "detail": detail })))
        };
        return Err(
            friendly_http_status_error(api, route, &url, status.as_u16(), body_json.as_ref())
                .into(),
        );
    }
    let v: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
        format!(
            "response for {} ({}) was HTTP {} but not valid JSON: {}; body prefix: {:.200}",
            route, url, status.as_u16(), e, body_text
        )
    })?;
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
        "raw",
        "raw/images",
        "reports",
        "slides",
        "lint",
        "scratch",
    ] {
        fs::create_dir_all(b.join(rel))?;
    }
    Ok(())
}

fn load_agent_doc(
    workspace_root: &Path,
    step: &str,
) -> Result<AgentDoc<StageAgentMeta>, Box<dyn std::error::Error>> {
    let path = workspace_root.join("agents").join(format!("{step}.md"));
    let sm = parse_stage_agent(&path).map_err(|e| e.to_string())?;
    Ok(AgentDoc {
        meta: sm,
        path,
    })
}

fn collect_step_paths(
    workspace_root: &Path,
    workdir: &Path,
) -> Result<BTreeMap<String, PathBuf>, Box<dyn std::error::Error>> {
    let (_, agents) = load_spec(workspace_root)?;
    let mut map = BTreeMap::new();
    for (name, agent) in agents {
        if let Some(rel) = &agent.meta.output {
            let p = workdir.join(rel);
            if p.exists() {
                map.insert(name, p);
            }
        }
    }
    Ok(map)
}

#[allow(clippy::too_many_arguments)]
fn run_llm_stage(
    api: &str,
    app_root: &Path,
    workdir: &Path,
    agent: &AgentDoc<StageAgentMeta>,
    step: &str,
    extra_user_block: &str,
    step_paths: &BTreeMap<String, PathBuf>,
    previous_artifact: Option<&Path>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let rel = agent
        .meta
        .output
        .as_deref()
        .ok_or_else(|| format!("stage `{step}` missing `output` in {}", agent.path.display()))?;
    let out_path = workdir.join(rel);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let prompt_path = app_root.join(
        agent
            .meta
            .prompt_file
            .as_deref()
            .unwrap_or("prompts/stage.md"),
    );
    let prompt = read_or_empty(&prompt_path);
    let ctx = render_context_attachments(
        workdir,
        &agent.meta.context_paths,
        step_paths,
        previous_artifact,
    )
    .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;
    let msg = if extra_user_block.trim().is_empty() {
        format!("{prompt}\n\n{ctx}")
    } else {
        format!("{prompt}\n\n{extra_user_block}\n\n{ctx}")
    };
    let reply = chat_no_tools(api, &msg)?;
    let reply = maybe_normalize_markdown(&agent.meta, &reply);
    fs::write(&out_path, reply)?;
    Ok(out_path)
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

    let run_stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let log_file = work_debug(&work)
        .join("scratch")
        .join(format!("orchestration-{run_stamp}.md"));
    let mut log = String::new();
    let mut task_outputs: Vec<(String, PathBuf)> = Vec::new();
    let mut artifacts = RunArtifacts::default();
    let edge_slice = if main.meta.edges.is_empty() {
        None
    } else {
        Some(main.meta.edges.as_slice())
    };
    let graph = resolve_execution_graph(&main.meta.pipeline, edge_slice)
        .map_err(|e| e.to_string())?;
    let schedule = execution_order(&graph);
    let total_steps = schedule.len();
    let last_step = schedule.last().cloned();
    let run_title = main
        .meta
        .display_name
        .as_deref()
        .unwrap_or(&main.meta.id);
    log.push_str("# Agent App Run\n\n");
    log.push_str(&format!(
        "- App: {} ({})\n- Source: {}\n- Question: {}\n\n",
        run_title, main.meta.id, source, q
    ));
    println!("Starting staged app: {} ({})", run_title, main.meta.id);
    println!("Task count: {}", total_steps);

    let mut step_paths: BTreeMap<String, PathBuf> = BTreeMap::new();

    for (idx, step) in schedule.iter().enumerate() {
        let agent = agents
            .get(step)
            .ok_or_else(|| format!("missing step agent: {step}"))?;
        let prev_path = single_predecessor(&graph, step).and_then(|pred| step_paths.get(&pred).cloned());
        println!(
            "[{}/{}] {} ({})",
            idx + 1,
            total_steps,
            step,
            agent.meta.kind
        );
        log.push_str(&format!("## Step: {} ({})\n\n", step, agent.meta.kind));

        let out_path = if agent.meta.kind == "ingest" {
            let p = ingest_assets_markdown(&work_debug(&work), source)?;
            log.push_str(&format!("- output: {}\n\n", p.display()));
            task_outputs.push((step.clone(), p.clone()));
            on_step(step, agent.meta.kind.as_str(), p.as_path());
            println!("  -> {}", p.display());
            p
        } else {
            let extra = if agent.meta.kind == "ask" {
                format!("User question:\n{q}\n")
            } else {
                String::new()
            };
            let op = run_llm_stage(
                api,
                &root,
                &work,
                agent,
                step,
                &extra,
                &step_paths,
                prev_path.as_deref(),
            )?;
            log.push_str(&format!("- output: {}\n\n", op.display()));
            task_outputs.push((step.clone(), op.clone()));
            let is_last = Some(step.as_str()) == last_step.as_deref();
            if is_last {
                artifacts.handoff = Some(op.clone());
            } else if agent.meta.kind != "ingest" {
                if artifacts.summary.is_none() {
                    artifacts.summary = Some(op.clone());
                } else if artifacts.report.is_none() {
                    artifacts.report = Some(op.clone());
                }
            }
            on_step(step, agent.meta.kind.as_str(), op.as_path());
            println!("  -> {}", op.display());
            op
        };
        step_paths.insert(step.clone(), out_path);
    }

    fs::write(&log_file, log)?;
    println!("\nSub-agent execution trace:");
    for (task, out) in &task_outputs {
        println!("- {} -> {}", task, out.display());
    }

    println!("\nFinal output artifacts:");
    if let Some(p) = &artifacts.summary {
        println!("- summary stage: {}", p.display());
    }
    if let Some(p) = &artifacts.report {
        println!("- report stage: {}", p.display());
    }
    if let Some(p) = &artifacts.handoff {
        println!("- final handoff: {}", p.display());
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
            Some(&v),
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
    body: Option<&serde_json::Value>,
) -> String {
    let mut msg = format!(
        "request failed for {} ({}): HTTP {}",
        route, url, status_code
    );
    if let Some(v) = body {
        if let Some(d) = v.get("detail").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
            msg.push_str("\nServer detail:\n");
            msg.push_str(d);
        } else if *v != serde_json::json!({}) {
            msg.push_str(&format!("\nResponse body: {}", v));
        }
    }
    msg.push_str("\nPossible root causes:");
    if status_code == 404 {
        msg.push_str("\n- Endpoint is missing (version mismatch or wrong API URL).");
    } else if status_code >= 500 {
        msg.push_str("\n- Kowalski server hit an error while handling the request (see \"Server detail\" above if present).");
        msg.push_str("\n- Upstream LLM (Ollama/OpenAI) unreachable, wrong host/port, or model missing.");
        if route == "/api/chat" {
            msg.push_str("\n- For Ollama: run `ollama serve` (or the desktop app), then `curl -s http://127.0.0.1:11434/api/tags` and `ollama pull <model>` for the model in the server's `config.toml` `[llm]` / `[ollama]`.");
        }
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
    // reqwest blocking `Client` defaults to a 30s **overall** timeout; idle SSE
    // reads then fail with `error decoding response body` (Decode kind) — disable it here.
    let client = reqwest_blocking::Client::builder()
        .timeout(None::<std::time::Duration>)
        .build()?;
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
    let agent = load_agent_doc(workspace_root, &instr.step)?;
    let raw_dir = work_debug(workdir).join("raw");
    let prev_owned = instr
        .previous_artifact
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| latest_md_in(&raw_dir));
    let prev = prev_owned.as_deref();
    if prev.is_none() {
        return Err("compile: no input artifact available (run ingest first)".into());
    }
    publish_agent_message(
        api,
        topic,
        agent_id,
        instr,
        &format!("LLM stage `{}` (federation worker)", instr.step),
    );
    let step_paths = collect_step_paths(workspace_root, workdir)?;
    let out = run_llm_stage(
        api,
        workspace_root,
        workdir,
        &agent,
        &instr.step,
        "",
        &step_paths,
        prev,
    )?;
    Ok((
        out.display().to_string(),
        format!("stage output: {}", out.display()),
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
    let agent = load_agent_doc(workspace_root, &instr.step)?;
    let q = instr
        .question
        .clone()
        .unwrap_or_else(|| "What changed in the latest source?".to_string());
    publish_agent_message(
        api,
        topic,
        agent_id,
        instr,
        &format!("LLM stage `{}` (federation): {}", instr.step, q),
    );
    let step_paths = collect_step_paths(workspace_root, workdir)?;
    let prev = instr.previous_artifact.as_deref().map(Path::new);
    let extra = format!("User question:\n{q}\n");
    let out = run_llm_stage(
        api,
        workspace_root,
        workdir,
        &agent,
        &instr.step,
        &extra,
        &step_paths,
        prev,
    )?;
    Ok((
        out.display().to_string(),
        format!("stage output: {}", out.display()),
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
    let agent = load_agent_doc(workspace_root, &instr.step)?;
    publish_agent_message(
        api,
        topic,
        agent_id,
        instr,
        &format!("LLM stage `{}` (federation handoff)", instr.step),
    );
    let step_paths = collect_step_paths(workspace_root, workdir)?;
    let prev = instr.previous_artifact.as_deref().map(Path::new);
    let out = run_llm_stage(
        api,
        workspace_root,
        workdir,
        &agent,
        &instr.step,
        "",
        &step_paths,
        prev,
    )?;
    Ok((
        out.display().to_string(),
        format!("handoff output: {}", out.display()),
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
                        .or_else(|| latest_md_in(&work_debug(&work).join("reports")))
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    let handoff_disp = done
                        .handoff
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
                        "run complete; summary={}; report={}; handoff={}; log={}",
                        summary, report, handoff_disp, log
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
        latest_md_in(&work_debug(&work).join("reports"))
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none yet)".to_string())
    );
    println!(
        "- stage outputs (see manifest `agents/*.md` `output` paths): {}",
        work_debug(&work).display()
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
