use crate::error::KowalskiCliError;
use kowalski_core::config::McpConfig;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Load only the `[mcp]` section from a TOML file. Other top-level tables (e.g. `[ollama]`) are ignored.
#[derive(Debug, Deserialize)]
struct McpSection {
    #[serde(default)]
    mcp: McpConfig,
}

/// Read `[mcp]` from `path` (e.g. workspace `config.toml`). Missing `[mcp]` yields empty `servers`.
pub fn load_mcp_config_from_file(path: &Path) -> Result<McpConfig, KowalskiCliError> {
    let content = fs::read_to_string(path).map_err(|e| {
        KowalskiCliError::Config(format!("Failed to read {}: {}", path.display(), e))
    })?;
    let section: McpSection = toml::from_str(&content).map_err(|e| {
        KowalskiCliError::Config(format!("Failed to parse TOML {}: {}", path.display(), e))
    })?;
    Ok(section.mcp)
}
