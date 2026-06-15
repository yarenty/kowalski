//! `kowalski-mcp-rookery` — stdio MCP server for the Rookery horde builder.
//!
//! Reads newline-delimited JSON-RPC from stdin and writes one JSON reply per request to
//! stdout. Logs go to **stderr** only (stdout is the protocol stream and must stay clean).

use clap::Parser;
use kowalski_mcp_rookery::dispatch;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Kowalski Rookery MCP server (stdio): validate + give birth to hordes"
)]
struct Cli {
    /// Default output root for `rookery_give_birth` when the call omits `output_root`.
    #[arg(long, default_value = "examples")]
    output_root: PathBuf,
}

fn main() -> io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    log::info!(
        "kowalski-mcp-rookery starting (output_root={})",
        cli.output_root.display()
    );

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let body: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": serde_json::Value::Null,
                    "error": { "code": -32700, "message": format!("parse error: {e}") }
                });
                writeln!(out, "{err}")?;
                out.flush()?;
                continue;
            }
        };

        if let Some(reply) = dispatch(&body, &cli.output_root) {
            writeln!(out, "{}", serde_json::to_string(&reply)?)?;
            out.flush()?;
        }
    }

    Ok(())
}
