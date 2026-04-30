use clap::Parser;
use datafusion::prelude::*;
use kowalski_mcp_datafusion::{ACCEPT_STREAMABLE, AppState, app_router};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "kowalski-mcp-datafusion")]
#[command(about = "MCP Streamable HTTP: SQL, schema, and column stats (DataFusion + CSV)")]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: String,
    #[arg(long)]
    csv: PathBuf,
    #[arg(long, default_value = "data")]
    table: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if !args.csv.exists() {
        return Err(format!("CSV not found: {}", args.csv.display()).into());
    }

    let ctx = SessionContext::new();
    let path = args.csv.to_str().ok_or("CSV path must be valid UTF-8")?;
    ctx.register_csv(&args.table, path, CsvReadOptions::new())
        .await?;

    let session_id = uuid::Uuid::new_v4().to_string();
    let state = AppState::new(Arc::new(ctx), args.table.clone(), session_id.clone());

    let app = app_router(state);
    let addr: SocketAddr = args.bind.parse()?;
    eprintln!(
        "kowalski-mcp-datafusion: table `{}` <- `{}` | session {} | listening http://{}",
        args.table,
        args.csv.display(),
        session_id,
        addr
    );
    eprintln!("Accept header for clients: `{}`", ACCEPT_STREAMABLE);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
