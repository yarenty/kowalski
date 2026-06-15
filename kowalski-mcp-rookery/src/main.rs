//! `kowalski-mcp-rookery` — MCP server for the Rookery horde builder.
//!
//! Two transports, same handler ([`RookeryHandler`]):
//! - `--transport stdio` (default): newline-delimited JSON-RPC on stdin/stdout.
//! - `--transport http`: **stateless** Streamable HTTP on `--bind` (no `Mcp-Session-Id`).
//!
//! Logs go to **stderr** only — under stdio, stdout is the protocol stream.

use clap::{Parser, ValueEnum};
use kowalski_mcp_rookery::RookeryHandler;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum Transport {
    /// Newline-delimited JSON-RPC on stdin/stdout (for desktop MCP clients).
    Stdio,
    /// Stateless Streamable HTTP (JSON or SSE) on `--bind`.
    Http,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Kowalski Rookery MCP server: validate + give birth to hordes (stdio or stateless HTTP)"
)]
struct Cli {
    /// Transport to serve on.
    #[arg(long, value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,

    /// Address to bind when `--transport http`.
    #[arg(long, default_value = "127.0.0.1:8081")]
    bind: String,

    /// Default output root for `rookery_give_birth` when the call omits `output_root`.
    #[arg(long, default_value = "examples")]
    output_root: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    let handler = Arc::new(RookeryHandler::new(cli.output_root.clone()));

    match cli.transport {
        Transport::Stdio => {
            log::info!(
                "kowalski-mcp-rookery: stdio transport (output_root={})",
                cli.output_root.display()
            );
            kowalski_mcp_transport::run_stdio(handler).await
        }
        Transport::Http => {
            let addr: SocketAddr = cli.bind.parse().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("invalid --bind '{}': {e}", cli.bind),
                )
            })?;
            eprintln!(
                "kowalski-mcp-rookery: stateless Streamable HTTP on http://{addr} (output_root={})",
                cli.output_root.display()
            );
            eprintln!(
                "Accept header for clients: `{}`",
                kowalski_mcp_transport::ACCEPT_STREAMABLE
            );
            kowalski_mcp_transport::serve_http(addr, handler).await
        }
    }
}
