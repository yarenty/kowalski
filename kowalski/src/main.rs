use clap::Parser;

mod auth;
mod horde;
mod http_api;
mod http_ops;
mod rookery;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Kowalski server",
    long_about = "Run the Kowalski HTTP API server used by the UI."
)]
struct Cli {
    /// Listen address (default from `kowalski_core::config::DEFAULT_API_BIND`,
    /// which `ui/vite.config.ts` proxy and the CLI default mirror)
    #[clap(long, default_value = kowalski_core::config::DEFAULT_API_BIND)]
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
    /// Enable API bearer-token auth + CORS origin allowlist. Off by default
    /// (single-user local tool). Also enabled by `[server] auth = true` in the
    /// config or a non-empty `KOWALSKI_API_TOKEN` env var.
    #[clap(long)]
    auth: bool,
    /// Allowed browser origin for CORS (repeatable). Defaults to the Vite dev UI
    /// origins. Only used with auth enabled (otherwise CORS is permissive).
    #[clap(long = "cors-origin", value_name = "ORIGIN")]
    cors_origins: Vec<String>,
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
    let security = http_api::SecurityOptions {
        auth: cli.auth,
        cors_origins: cli.cors_origins,
    };
    http_api::serve(addr, cli.config, cli.ollama_url, tls, security).await?;

    Ok(())
}
