use clap::Parser;

mod horde;
mod http_api;
mod http_ops;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Kowalski server",
    long_about = "Run the Kowalski HTTP API server used by the UI."
)]
struct Cli {
    /// Listen address (default 127.0.0.1:3456 — matches `ui/vite.config.ts` proxy)
    #[clap(long, default_value = "127.0.0.1:3456")]
    bind: String,
    /// Config TOML path (default ./config.toml)
    #[clap(short, long)]
    config: Option<String>,
    /// Ollama base URL for `/api/doctor` (default http://127.0.0.1:11434)
    #[clap(long)]
    ollama_url: Option<String>,
    /// TLS certificate (PEM). Must be set together with `--tls-key`.
    #[clap(long, value_name = "PEM")]
    tls_cert: Option<std::path::PathBuf>,
    /// TLS private key (PEM). Must be set together with `--tls-cert`.
    #[clap(long, value_name = "PEM")]
    tls_key: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    let addr: std::net::SocketAddr = cli
        .bind
        .parse()
        .map_err(|e| format!("Invalid --bind {:?}: {}", cli.bind, e))?;
    let tls = match (cli.tls_cert, cli.tls_key) {
        (Some(c), Some(k)) => Some((c, k)),
        (None, None) => None,
        _ => {
            return Err("--tls-cert and --tls-key must be set together (or both omitted)".into());
        }
    };
    http_api::serve(addr, cli.config, cli.ollama_url, tls).await?;

    Ok(())
}
