use kowalski_core::config::{Config, McpConfig, McpTransport};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct McpSection {
    #[serde(default)]
    mcp: McpConfig,
}

fn load_mcp_config_from_file(path: &Path) -> Result<McpConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let section: McpSection = toml::from_str(&content)?;
    Ok(section.mcp)
}

pub fn mcp_config_path(config_path: Option<&str>) -> PathBuf {
    config_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

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

#[derive(Debug, Clone, Serialize)]
pub struct McpServerPublic {
    pub name: String,
    pub url: String,
    pub transport: String,
}

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
    pub server_version: String,
    pub ollama: OllamaProbeJson,
    pub llm: LlmDoctorJson,
    pub operator: DoctorOperatorJson,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorOperatorJson {
    pub mcp_servers_configured: usize,
    pub postgres_memory_configured: bool,
    pub config_divergence: Vec<String>,
    pub mcp_streamable_session_note: &'static str,
}

pub fn config_divergence_lines(c: &Config) -> Vec<String> {
    let d = Config::default();
    let mut v = Vec::new();
    if c.ollama.model != d.ollama.model {
        v.push("ollama.model".into());
    }
    if c.ollama.host != d.ollama.host || c.ollama.port != d.ollama.port {
        v.push("ollama host/port".into());
    }
    if c.memory.database_url.is_some() {
        v.push("memory.database_url set".into());
    }
    if c.memory.episodic_path != d.memory.episodic_path {
        v.push("memory.episodic_path".into());
    }
    if !c.mcp.servers.is_empty() {
        v.push(format!("mcp.servers: {}", c.mcp.servers.len()));
    }
    if c.llm.provider != d.llm.provider {
        v.push("llm.provider".into());
    }
    if c.llm.openai_api_base != d.llm.openai_api_base {
        v.push("llm.openai_api_base".into());
    }
    v
}

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

pub fn list_mcp_servers_public(path: &Path) -> Result<Vec<McpServerPublic>, Box<dyn std::error::Error>> {
    let mcp = load_mcp_config_from_file(path)?;
    Ok(
        mcp.servers
            .iter()
            .map(|s| McpServerPublic {
                name: s.name.clone(),
                url: s.url.clone(),
                transport: match s.transport {
                    McpTransport::Http => "http".to_string(),
                    McpTransport::Sse => "sse".to_string(),
                    McpTransport::Stdio => "stdio".to_string(),
                },
            })
            .collect(),
    )
}

pub async fn mcp_ping_results(path: &Path) -> Result<Vec<McpPingResult>, Box<dyn std::error::Error>> {
    let mcp = load_mcp_config_from_file(path)?;
    let mut out = Vec::with_capacity(mcp.servers.len());
    for server in &mcp.servers {
        let transport = match server.transport {
            McpTransport::Http => "http",
            McpTransport::Sse => "sse",
            McpTransport::Stdio => "stdio",
        };
        let url_display = if server.url.trim().is_empty() {
            server.command.join(" ")
        } else {
            server.url.clone()
        };
        let result: Result<
            Vec<kowalski_core::mcp::types::McpToolDescription>,
            kowalski_core::KowalskiError,
        > = if matches!(server.transport, McpTransport::Stdio) {
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

pub async fn doctor_json(ollama_base: Option<String>, config: Option<&Config>) -> DoctorJson {
    let base = ollama_base.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    let ollama = probe_ollama_tags(&base).await;
    let c = config.cloned().unwrap_or_default();
    let llm = LlmDoctorJson {
        provider: c.llm.provider.clone(),
        model: c.ollama.model.clone(),
        openai_api_base: c.llm.openai_api_base.clone(),
    };
    let operator = DoctorOperatorJson {
        mcp_servers_configured: c.mcp.servers.len(),
        postgres_memory_configured: kowalski_core::config::memory_uses_postgres(&c.memory),
        config_divergence: config_divergence_lines(&c),
        mcp_streamable_session_note: "After initialize, Streamable HTTP MCP session ids are available via `McpClient::session_id()` (and `McpClient::shutdown()` clears the session).",
    };
    DoctorJson {
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        ollama,
        llm,
        operator,
    }
}
