//! Operator commands: config validation, DB migrations, environment checks.

use kowalski_core::config::Config;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Default path for `[mcp]` and full config TOML (CLI and HTTP API).
pub fn mcp_config_path(config_path: Option<&str>) -> PathBuf {
    config_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

/// Load full [`Config`] for `kowalski-cli serve` (HTTP chat + MCP). Missing file → [`Config::default`].
pub fn load_kowalski_config_for_serve(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    if !path.exists() {
        log::warn!(
            "No config at {} — using defaults (Ollama localhost; add config.toml for MCP/tools)",
            path.display()
        );
        return Ok(Config::default());
    }
    let raw = fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

/// Public MCP server metadata for JSON APIs (no auth headers).
#[derive(Debug, Clone, Serialize)]
pub struct McpServerPublic {
    pub name: String,
    pub url: String,
    pub transport: String,
}

/// Result of probing one MCP server (initialize + tools/list).
#[derive(Debug, Clone, Serialize)]
pub struct McpPingResult {
    pub name: String,
    pub url: String,
    pub transport: String,
    pub ok: bool,
    pub tool_count: Option<usize>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorJson {
    pub cli_version: String,
    pub ollama: OllamaProbeJson,
    /// From `[llm]` + `[ollama].model` (no API keys).
    pub llm: LlmDoctorJson,
}

/// Non-secret LLM routing snapshot for `/api/doctor`.
#[derive(Debug, Clone, Serialize)]
pub struct LlmDoctorJson {
    pub provider: String,
    pub model: String,
    pub openai_api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OllamaProbeJson {
    pub url: String,
    pub ok: bool,
    pub detail: String,
}

/// List `[mcp.servers]` entries from TOML (headers omitted).
pub fn list_mcp_servers_public(path: &Path) -> Result<Vec<McpServerPublic>, Box<dyn std::error::Error>> {
    use crate::config::load_mcp_config_from_file;

    let mcp = load_mcp_config_from_file(path)?;
    Ok(
        mcp.servers
            .iter()
            .map(|s| McpServerPublic {
                name: s.name.clone(),
                url: s.url.clone(),
                transport: match s.transport {
                    kowalski_core::config::McpTransport::Http => "http".to_string(),
                    kowalski_core::config::McpTransport::Sse => "sse".to_string(),
                    kowalski_core::config::McpTransport::Stdio => "stdio".to_string(),
                },
            })
            .collect(),
    )
}

/// Run initialize + tools/list for each configured MCP server.
pub async fn mcp_ping_results(path: &Path) -> Result<Vec<McpPingResult>, Box<dyn std::error::Error>> {
    use crate::config::load_mcp_config_from_file;

    let mcp = load_mcp_config_from_file(path)?;
    let mut out = Vec::with_capacity(mcp.servers.len());
    for server in &mcp.servers {
        let transport = match server.transport {
            kowalski_core::config::McpTransport::Http => "http",
            kowalski_core::config::McpTransport::Sse => "sse",
            kowalski_core::config::McpTransport::Stdio => "stdio",
        };
        let url_display = if server.url.trim().is_empty() {
            server.command.join(" ")
        } else {
            server.url.clone()
        };
        let result: Result<
            Vec<kowalski_core::mcp::types::McpToolDescription>,
            kowalski_core::KowalskiError,
        > = if matches!(
            server.transport,
            kowalski_core::config::McpTransport::Stdio
        ) {
            match kowalski_core::McpStdioClient::connect(server).await {
                Ok(c) => c.list_tools().await,
                Err(e) => Err(e),
            }
        } else {
            match kowalski_core::mcp::McpClient::connect_server(server).await {
                Ok(c) => c.list_tools().await,
                Err(e) => Err(e),
            }
        };
        match result {
            Ok(tools) => out.push(McpPingResult {
                name: server.name.clone(),
                url: url_display,
                transport: transport.to_string(),
                ok: true,
                tool_count: Some(tools.len()),
                error: None,
            }),
            Err(e) => out.push(McpPingResult {
                name: server.name.clone(),
                url: url_display,
                transport: transport.to_string(),
                ok: false,
                tool_count: None,
                error: Some(e.to_string()),
            }),
        }
    }
    Ok(out)
}

async fn probe_ollama_tags(base: &str) -> OllamaProbeJson {
    let base = base.trim_end_matches('/');
    let tags_url = format!("{}/api/tags", base);
    match reqwest::get(&tags_url).await {
        Ok(r) => {
            if r.status().is_success() {
                OllamaProbeJson {
                    url: tags_url,
                    ok: true,
                    detail: format!("HTTP {}", r.status()),
                }
            } else {
                OllamaProbeJson {
                    url: tags_url,
                    ok: false,
                    detail: format!("HTTP {}", r.status()),
                }
            }
        }
        Err(e) => OllamaProbeJson {
            url: tags_url,
            ok: false,
            detail: e.to_string(),
        },
    }
}

/// JSON payload for `/api/doctor` (and similar UIs).
pub async fn doctor_json(ollama_base: Option<String>, config: Option<&Config>) -> DoctorJson {
    let base = ollama_base.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    let ollama = probe_ollama_tags(&base).await;
    let c = config.cloned().unwrap_or_default();
    let llm = LlmDoctorJson {
        provider: c.llm.provider.clone(),
        model: c.ollama.model.clone(),
        openai_api_base: c.llm.openai_api_base.clone(),
    };
    DoctorJson {
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        ollama,
        llm,
    }
}

/// Validate TOML and optionally a full [`Config`] parse.
pub fn run_config_check(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let _toml: toml::Value = toml::from_str(&raw)?;
    println!("OK — valid TOML ({})", path.display());

    match toml::from_str::<Config>(&raw) {
        Ok(c) => {
            println!("OK — parses as Kowalski core `Config`");
            println!("  ollama: {}:{} / model {}", c.ollama.host, c.ollama.port, c.ollama.model);
            println!(
                "  memory: episodic_path = {}",
                c.memory.episodic_path
            );
            if let Some(ref u) = c.memory.database_url {
                println!("  memory.database_url = {}", u);
            } else {
                println!("  memory.database_url = (unset — Tier 2 SQLite file only)");
            }
            println!("  mcp servers: {}", c.mcp.servers.len());
            println!(
                "  llm: provider = {}, model = {}",
                c.llm.provider, c.ollama.model
            );
            if let Some(ref b) = c.llm.openai_api_base {
                println!("  llm.openai_api_base = {}", b);
            }
        }
        Err(e) => {
            println!("Note — not a full core `Config` (fix or use partial TOML only):");
            println!("  {}", e);
        }
    }
    Ok(())
}

/// Run `memory.database_url` migrations from `--url` or from `memory.database_url` in TOML.
pub async fn run_db_migrate(
    url: Option<String>,
    config: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = if let Some(u) = url {
        u
    } else {
        let path = config
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("config.toml"));
        let raw = fs::read_to_string(&path)?;
        let v: toml::Value = toml::from_str(&raw)?;
        let url = v
            .get("memory")
            .and_then(|m| m.get("database_url"))
            .and_then(|x| x.as_str())
            .ok_or("No memory.database_url in config and no --url")?;
        url.to_string()
    };

    println!("Running migrations for {}", resolved);
    kowalski_core::db::run_migrations(&resolved).await?;
    println!("Done.");
    Ok(())
}

/// Print versions and probe local Ollama (default `http://127.0.0.1:11434`).
pub async fn run_doctor(ollama_base: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_optional_config_default_path();
    let j = doctor_json(ollama_base, config.as_ref()).await;
    println!("kowalski-cli {}", j.cli_version);
    println!(
        "LLM: provider = {}, model = {}",
        j.llm.provider, j.llm.model
    );
    if let Some(ref b) = j.llm.openai_api_base {
        println!("LLM: openai_api_base = {}", b);
    }
    if j.ollama.ok {
        println!("Ollama: OK — {}", j.ollama.url);
    } else if j.ollama.detail.starts_with("HTTP ") {
        println!("Ollama: {} — {}", j.ollama.detail, j.ollama.url);
    } else {
        println!("Ollama: unreachable ({}) — {}", j.ollama.url, j.ollama.detail);
    }
    Ok(())
}

fn load_optional_config_default_path() -> Option<Config> {
    let path = PathBuf::from("config.toml");
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(&path).ok()?;
    toml::from_str(&raw).ok()
}
