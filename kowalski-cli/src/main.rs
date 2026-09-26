use clap::Parser;
use std::io::{self, Write};

use kowalski_core::memory::consolidation::{Consolidator, MemoryWeaver};

use kowalski_core::config::default_model as determine_model;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Kowalski CLI — chat with an agent, run hordes, and operate memory and MCP.",
    long_about = "`kowalski-cli` with no command (or `chat`, `run`) starts a chat with an agent that uses your config's model and MCP tools. Hordes: `agent-app`. Operators: `config check`, `db migrate`, `doctor`, `mcp ping`, `mcp tools`, `federation ping-notify` (with `--features postgres`). See --help on each."
)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,

    /// Config TOML for the default chat (default: $KOWALSKI_CONFIG, ./config.toml, then ~/.config/kowalski/config.toml)
    #[clap(short, long)]
    config: Option<String>,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Consolidate memory - move from episodic history into semantic memory
    Consolidate {
        #[clap(long)]
        delete: bool,
        /// Config TOML (default: $KOWALSKI_CONFIG, ./config.toml, then ~/.config/kowalski/config.toml)
        #[clap(short, long)]
        config: Option<String>,
    },
    /// Model Context Protocol helpers
    Mcp {
        #[clap(subcommand)]
        command: McpCommands,
    },
    /// Validate configuration TOML (and full Kowalski `Config` when possible)
    Config {
        #[clap(subcommand)]
        command: ConfigCommands,
    },
    /// Run SQL migrations for `sqlite:` or `postgres://` URLs
    Db {
        #[clap(subcommand)]
        command: DbCommands,
    },
    /// Print versions and probe local Ollama
    Doctor {
        /// Ollama base URL (default http://127.0.0.1:11434)
        #[clap(long)]
        ollama_url: Option<String>,
    },
    /// Chat with an agent that uses your config's model and MCP tools (also: `chat`, or no command)
    #[clap(alias = "chat")]
    Run {
        /// Config TOML (default: $KOWALSKI_CONFIG, ./config.toml, then ~/.config/kowalski/config.toml)
        #[clap(short, long)]
        config: Option<String>,
    },
    /// Federation operators (Postgres `NOTIFY` smoke test when built with `--features postgres`)
    Federation {
        #[clap(subcommand)]
        command: FederationCommands,
    },
    /// Run extension commands discovered from PATH or local extension directory
    Extension {
        #[clap(subcommand)]
        command: ExtensionCommands,
    },
    /// Markdown-defined app agents (main + sub-agents in .md files)
    AgentApp {
        #[clap(subcommand)]
        command: AgentAppCommands,
    },
}

#[derive(Parser, Debug)]
enum ConfigCommands {
    /// Check that TOML parses and optionally matches core `Config`
    Check {
        /// Path to config.toml (default: config.toml)
        #[clap(default_value = "config.toml")]
        path: String,
    },
}

#[derive(Parser, Debug)]
enum DbCommands {
    /// Apply embedded migrations to the database URL (`memory.database_url` or `--url`)
    Migrate {
        /// SQL store URL (sqlite:… or postgres://…)
        #[clap(long)]
        url: Option<String>,
        /// Read memory.database_url from this TOML (ignored if --url is set)
        #[clap(short, long)]
        config: Option<String>,
    },
}

#[derive(Parser, Debug)]
enum FederationCommands {
    /// Send a Ping ACL via `pg_notify` on `kowalski_federation` (needs `memory.database_url` in config)
    PingNotify {
        /// Config TOML (default: $KOWALSKI_CONFIG, ./config.toml, then ~/.config/kowalski/config.toml)
        #[clap(short, long)]
        config: Option<String>,
    },
}

#[derive(Parser, Debug)]
enum McpCommands {
    /// Run initialize + tools/list against each server in [mcp] (from config TOML)
    Ping {
        /// TOML file containing an [mcp] section (default: ./config.toml)
        #[clap(short, long)]
        config: Option<String>,
    },
    /// List tool names and descriptions per MCP server (same config as ping)
    Tools {
        /// TOML file containing an [mcp] section (default: ./config.toml)
        #[clap(short, long)]
        config: Option<String>,
    },
}

#[derive(Parser, Debug)]
enum ExtensionCommands {
    /// List available extensions (PATH `kowalski-ext-*` and local `.kowalski/extensions/*`)
    List,
    /// Run an extension by name, forwarding trailing arguments as-is
    Run {
        /// Extension name (for binary `kowalski-ext-<name>`)
        name: String,
        /// Arguments forwarded to extension command
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Parser, Debug)]
enum AgentAppCommands {
    /// List horde pipeline and agents
    List {
        /// App dir (`horde.md` + `agents/`). Env `KOWALSKI_AGENT_APP_ROOT`, else dev default `examples/knowledge-compiler`.
        #[clap(short, long)]
        path: Option<String>,
    },
    /// Validate `horde.md` + `agents/*.md` (pipeline vs agent files)
    Validate {
        /// App dir (`horde.md` + `agents/`). Env `KOWALSKI_AGENT_APP_ROOT`, else dev default `examples/knowledge-compiler`.
        #[clap(short, long)]
        path: Option<String>,
    },
    /// Run pipeline sequentially (same steps as `horde.md` pipeline)
    Run {
        /// Source URL or text
        source: String,
        /// Optional question for query phase
        #[clap(short, long)]
        question: Option<String>,
        /// App dir (`horde.md` + `agents/`). Env `KOWALSKI_AGENT_APP_ROOT`, else dev default `examples/knowledge-compiler`.
        #[clap(short, long)]
        path: Option<String>,
        /// Kowalski API base URL (default: `KOWALSKI_API` env, else http://127.0.0.1:3456)
        #[clap(long)]
        api: Option<String>,
    },
    /// Delegate one orchestrated run via federation `/api/federation/delegate`
    Delegate {
        /// Required capability selector (e.g. `kc.run`)
        capability: String,
        /// Source URL or text
        source: String,
        /// Optional question for query phase
        #[clap(short, long)]
        question: Option<String>,
        /// Kowalski API base URL (default: `KOWALSKI_API` env, else http://127.0.0.1:3456)
        #[clap(long)]
        api: Option<String>,
    },
    /// Run a federated worker: either whole-app delegations (no `--role`) or a single pipeline
    /// step when `--role` matches a `kind` from the app’s `agents/*.md` (defined only by `--path`).
    Worker {
        /// Worker agent id
        agent_id: String,
        /// Directory containing `horde.md` and `agents/*.md` (defaults: env `KOWALSKI_AGENT_APP_ROOT`, else repo `examples/knowledge-compiler` for local dev only).
        #[clap(short, long)]
        path: Option<String>,
        /// Kowalski API base URL (default: `KOWALSKI_API` env, else http://127.0.0.1:3456)
        #[clap(long)]
        api: Option<String>,
        /// Federation topic (default: federation)
        #[clap(long)]
        topic: Option<String>,
        /// Pipeline step name from the app spec (`agents/*.md` → `kind`), e.g. the ingest step’s `kind`.
        /// Omit to accept whole-pipeline delegates for whatever capability the server sends.
        #[clap(long)]
        role: Option<String>,
        /// Override the registered capability; default derives from `--role` or from legacy whole-run mode.
        #[clap(long)]
        capability: Option<String>,
    },
    /// Execute one process-isolated horde step and exit: reads a JSON request on stdin,
    /// emits JSON event lines on stdout (spawned by the server for `isolation = "process"` steps).
    ExecStep {
        /// Config TOML resolving the LLM provider/model for LLM step kinds (default: config.toml)
        #[clap(long)]
        config: Option<String>,
    },
    /// Export a horde as a portable `<id>-<version>.kwf.zip` bundle
    Export {
        /// Horde directory path, or a horde id resolved across the horde roots
        horde: String,
        /// Bundle file path (`*.zip`) or destination directory (default: current directory)
        #[clap(short, long)]
        output: Option<String>,
        /// Print the result as JSON
        #[clap(long)]
        json: bool,
        /// Config TOML the horde roots resolve against (default: config.toml)
        #[clap(long)]
        config: Option<String>,
    },
    /// Import a `.kwf.zip` (or `.bbwf.zip`) bundle as a draft horde and print the portability report
    Import {
        /// Bundle file to import
        bundle: String,
        /// Hordes root to land the draft in (default: the user hordes root, never examples/)
        #[clap(long)]
        dir: Option<String>,
        /// Print the portability report as JSON
        #[clap(long)]
        json: bool,
        /// Config TOML resolving the destination root and the local tool/model checks (default: config.toml)
        #[clap(long)]
        config: Option<String>,
    },
    /// Print reproducible end-to-end federation proof-run checklist
    Proof {
        /// App dir (`horde.md` + `agents/`). Env `KOWALSKI_AGENT_APP_ROOT`, else dev default `examples/knowledge-compiler`.
        #[clap(short, long)]
        path: Option<String>,
        /// Kowalski API base URL (default: `KOWALSKI_API` env, else http://127.0.0.1:3456)
        #[clap(long)]
        api: Option<String>,
        /// Worker agent id
        #[clap(long)]
        agent_id: Option<String>,
        /// Capability to delegate
        #[clap(long)]
        capability: Option<String>,
        /// Source to delegate
        #[clap(long)]
        source: Option<String>,
        /// Question to delegate
        #[clap(long)]
        question: Option<String>,
    },
}

async fn run_mcp_ping(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    use kowalski_cli::config::load_mcp_config_from_file;

    let path = kowalski_cli::ops::mcp_config_path(config_path);
    let mcp = load_mcp_config_from_file(&path)?;
    if mcp.servers.is_empty() {
        println!(
            "No MCP servers under [mcp] in {}. Add [[mcp.servers]] entries (see comments in config.toml).",
            path.display()
        );
        return Ok(());
    }

    println!(
        "MCP ping — {} ({} server(s))\n",
        path.display(),
        mcp.servers.len()
    );

    let results = kowalski_cli::ops::mcp_ping_results(&path).await?;
    for r in results {
        print!("  {} <{}> [{}] ... ", r.name, r.url, r.transport);
        io::stdout().flush()?;
        if r.ok {
            println!("OK — {} tool(s)", r.tool_count.unwrap_or(0));
        } else {
            let err = r.error.as_deref().unwrap_or("");
            if err.starts_with("tools/list:") {
                println!("partial — {}", err);
            } else {
                println!("FAILED — {}", err);
            }
        }
    }
    Ok(())
}

async fn run_mcp_tools(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    use kowalski_cli::config::load_mcp_config_from_file;

    let path = kowalski_cli::ops::mcp_config_path(config_path);
    let mcp = load_mcp_config_from_file(&path)?;
    if mcp.servers.is_empty() {
        println!(
            "No MCP servers under [mcp] in {}. Add [[mcp.servers]] entries.",
            path.display()
        );
        return Ok(());
    }

    println!(
        "MCP tools — {} ({} server(s))\n",
        path.display(),
        mcp.servers.len()
    );

    for server in &mcp.servers {
        let loc = if server.url.trim().is_empty() {
            server.command.join(" ")
        } else {
            server.url.clone()
        };
        println!(
            "[{}] {} ({})",
            server.name,
            loc,
            match server.transport {
                kowalski_core::config::McpTransport::Http => "http",
                kowalski_core::config::McpTransport::Sse => "sse",
                kowalski_core::config::McpTransport::Stdio => "stdio",
            }
        );
        let tools_result = if matches!(server.transport, kowalski_core::config::McpTransport::Stdio)
        {
            match kowalski_core::McpStdioClient::connect(server).await {
                Ok(c) => c.list_tools().await,
                Err(e) => Err(e),
            }
        } else {
            match kowalski_core::mcp::McpClient::connect_server(server).await {
                Ok(client) => {
                    if let Some(sid) = client.session_id() {
                        println!("  Session: {}", sid);
                    }
                    client.list_tools().await
                }
                Err(e) => Err(e),
            }
        };
        match tools_result {
            Ok(tools) => {
                if tools.is_empty() {
                    println!("  (no tools reported)");
                }
                for t in &tools {
                    let desc = t.description.trim();
                    let short = if desc.len() > 120 {
                        format!("{}…", &desc[..120])
                    } else {
                        desc.to_string()
                    };
                    println!("  • {} — {}", t.name, short);
                }
            }
            Err(e) => println!("  FAILED: {}", e),
        }
        println!();
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Mcp { command }) => match command {
            McpCommands::Ping {
                config: config_path,
            } => {
                run_mcp_ping(config_path.as_deref()).await?;
            }
            McpCommands::Tools {
                config: config_path,
            } => {
                run_mcp_tools(config_path.as_deref()).await?;
            }
        },
        Some(Commands::Config { command }) => match command {
            ConfigCommands::Check { path } => {
                kowalski_cli::ops::run_config_check(std::path::Path::new(&path))?;
            }
        },
        Some(Commands::Db { command }) => match command {
            DbCommands::Migrate { url, config } => {
                kowalski_cli::ops::run_db_migrate(url, config).await?;
            }
        },
        Some(Commands::Doctor { ollama_url }) => {
            kowalski_cli::ops::run_doctor(ollama_url).await?;
        }
        Some(Commands::Run { config }) => {
            kowalski_cli::run_ops::run_orchestrator(config.as_deref()).await?;
        }
        Some(Commands::Federation { command }) => match command {
            FederationCommands::PingNotify { config } => {
                kowalski_cli::federation_ops::run_ping_notify(config.as_deref()).await?;
            }
        },
        Some(Commands::Extension { command }) => match command {
            ExtensionCommands::List => {
                let items = kowalski_cli::extension_ops::list_extensions()?;
                if items.is_empty() {
                    println!("No extensions found.");
                    println!(
                        "Install `kowalski-ext-<name>` in PATH or add `.kowalski/extensions/<name>/run`."
                    );
                } else {
                    println!("Available extensions:");
                    for name in items {
                        println!("- {}", name);
                    }
                }
            }
            ExtensionCommands::Run { name, args } => {
                kowalski_cli::extension_ops::run_extension(&name, &args)?;
            }
        },
        Some(Commands::AgentApp { command }) => match command {
            AgentAppCommands::List { path } => {
                kowalski_cli::agent_app_ops::list_agents(path.as_deref())?;
            }
            AgentAppCommands::Validate { path } => {
                kowalski_cli::agent_app_ops::validate(path.as_deref())?;
            }
            AgentAppCommands::Run {
                source,
                question,
                path,
                api,
            } => {
                let out = tokio::task::spawn_blocking(move || {
                    kowalski_cli::agent_app_ops::run(
                        path.as_deref(),
                        &source,
                        question.as_deref(),
                        api.as_deref(),
                    )
                    .map_err(|e| e.to_string())
                })
                .await?;
                if let Err(e) = out {
                    return Err(e.into());
                }
            }
            AgentAppCommands::Delegate {
                capability,
                source,
                question,
                api,
            } => {
                let out = tokio::task::spawn_blocking(move || {
                    kowalski_cli::agent_app_ops::federate_delegate(
                        api.as_deref(),
                        &capability,
                        &source,
                        question.as_deref(),
                    )
                    .map_err(|e| e.to_string())
                })
                .await?;
                if let Err(e) = out {
                    return Err(e.into());
                }
            }
            AgentAppCommands::Worker {
                agent_id,
                path,
                api,
                topic,
                role,
                capability,
            } => {
                let out = tokio::task::spawn_blocking(move || {
                    kowalski_cli::agent_app_ops::federate_worker(
                        path.as_deref(),
                        api.as_deref(),
                        &agent_id,
                        topic.as_deref(),
                        role.as_deref(),
                        capability.as_deref(),
                    )
                    .map_err(|e| e.to_string())
                })
                .await?;
                if let Err(e) = out {
                    return Err(e.into());
                }
            }
            AgentAppCommands::ExecStep { config } => {
                kowalski_cli::agent_app_ops::exec_step(config.as_deref()).await?;
            }
            AgentAppCommands::Export {
                horde,
                output,
                json,
                config,
            } => {
                kowalski_cli::agent_app_ops::export_horde(
                    &horde,
                    output.as_deref(),
                    json,
                    config.as_deref(),
                )?;
            }
            AgentAppCommands::Import {
                bundle,
                dir,
                json,
                config,
            } => {
                kowalski_cli::agent_app_ops::import_horde(
                    &bundle,
                    dir.as_deref(),
                    json,
                    config.as_deref(),
                )
                .await?;
            }
            AgentAppCommands::Proof {
                path,
                api,
                agent_id,
                capability,
                source,
                question,
            } => {
                let out = tokio::task::spawn_blocking(move || {
                    kowalski_cli::agent_app_ops::proof_check(
                        path.as_deref(),
                        api.as_deref(),
                        agent_id.as_deref(),
                        capability.as_deref(),
                        source.as_deref(),
                        question.as_deref(),
                    )
                    .map_err(|e| e.to_string())
                })
                .await?;
                if let Err(e) = out {
                    return Err(e.into());
                }
            }
        },
        Some(Commands::Consolidate { delete, config }) => {
            let path = kowalski_cli::ops::mcp_config_path(config.as_deref());
            let config = kowalski_cli::ops::load_kowalski_config_for_serve(&path)?;
            let model = determine_model(&config);

            // Create the appropriate LLM provider for consolidation
            let llm_provider: std::sync::Arc<dyn kowalski_core::llm::LLMProvider> =
                if config.llm.provider == "openai" {
                    let api_key = config.llm.openai_api_key.clone().unwrap_or_default();
                    let base = config.llm.openai_api_base.as_deref();
                    std::sync::Arc::new(kowalski_core::llm::OpenAIProvider::new(&api_key, base))
                } else {
                    std::sync::Arc::new(kowalski_core::llm::OllamaProvider::new(
                        &config.ollama.host,
                        config.ollama.port,
                    ))
                };

            kowalski_core::db::run_memory_migrations_if_configured(&config).await?;

            let mut weaver = Consolidator::new(&config.memory, llm_provider, &model).await?;
            weaver.run(delete).await?;
            println!("Memory consolidation complete. Model: {}", model);
        }
        None => {
            // no command: the chat, with the top-level --config if given
            kowalski_cli::run_ops::run_orchestrator(cli.config.as_deref()).await?;
        }
    }
    Ok(())
}
