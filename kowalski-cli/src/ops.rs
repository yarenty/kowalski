//! Operator commands: config validation, DB migrations, environment checks.

use kowalski_core::config::Config;
use std::fs;
use std::path::{Path, PathBuf};

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
    println!("kowalski-cli {}", env!("CARGO_PKG_VERSION"));

    let base = ollama_base.unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    let tags_url = format!("{}/api/tags", base.trim_end_matches('/'));
    match reqwest::get(&tags_url).await {
        Ok(r) => {
            if r.status().is_success() {
                println!("Ollama: OK — {}", tags_url);
            } else {
                println!("Ollama: HTTP {} — {}", r.status(), tags_url);
            }
        }
        Err(e) => println!("Ollama: unreachable ({}) — {}", tags_url, e),
    }
    Ok(())
}
