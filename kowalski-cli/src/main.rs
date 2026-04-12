use clap::Parser;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::tools::ToolCall;
use log::info;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

use kowalski_core::memory::consolidation::{Consolidator, MemoryWeaver};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,

    /// Start in interactive mode
    #[clap(short, long)]
    interactive: bool,

    /// Path to a configuration file (.toml) to load an agent
    #[clap(short, long)]
    config: Option<String>,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Create a new agent
    Create {
        /// Agent type (web, academic, code, data)
        agent_type: String,
        /// Optional system prompt
        #[clap(short, long)]
        prompt: Option<String>,
        /// Optional temperature
        #[clap(short, long)]
        temperature: Option<f32>,
        /// Optional agent name
        #[clap(short, long)]
        name: Option<String>,
        /// Optional configuration file
        #[clap(short, long)]
        config: Option<String>,
    },
    /// Chat with an agent
    Chat {
        /// Agent name or type
        agent: String,
        /// Optional system prompt
        #[clap(short, long)]
        prompt: Option<String>,
        /// Optional temperature
        #[clap(short, long)]
        temperature: Option<f32>,
        /// Optional model
        #[clap(short, long)]
        model: Option<String>,
    },
    /// List available agent types
    List,
    /// List active agents
    Agents,
    /// Consolidate memory - move from episodic history into semantic memory
    Consolidate {
        #[clap(long)]
        delete: bool,
    },
    /// Model Context Protocol helpers
    Mcp {
        #[clap(subcommand)]
        command: McpCommands,
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
}

struct AgentManager {
    agents: Arc<RwLock<HashMap<String, Box<dyn Agent + Send + Sync>>>>,
    configs: Arc<RwLock<HashMap<String, Config>>>,
}

impl AgentManager {
    fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_agent_from_config(
        &self,
        config_path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use kowalski_cli::config::AgentConfig;
        use std::path::Path;

        let agent_config = AgentConfig::load_from_file(Path::new(config_path))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        println!(
            "Loading agent '{}' of type '{}'...",
            agent_config.name, agent_config.agent_type
        );

        self.create_agent(
            agent_config.name.clone(),
            &agent_config.agent_type,
            agent_config.system_prompt.as_deref(),
            agent_config.temperature,
        )
        .await?;

        // If tools are specified, we might need a way to register them after creation
        // but for now create_agent uses default tools for each type.
        // In the future, we'll use tool_manager directly.

        Ok(agent_config.name)
    }

    async fn create_agent(
        &self,
        name: String,
        agent_type: &str,
        _prompt: Option<&str>,
        _temperature: Option<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        use kowalski_core::template::default::DefaultTemplate;
        let builder = DefaultTemplate::create_agent(vec![], None, Some(0.7)).await?;
        let mut template_agent = builder.build().await?;
        template_agent.base_mut().set_system_prompt(&format!(
            "Starting generic agent (was requested type: {})",
            agent_type
        ));

        let agent: Box<dyn Agent + Send + Sync> = Box::new(template_agent);
        self.agents.write().await.insert(name.clone(), agent);
        self.configs.write().await.insert(name, config);
        Ok(())
    }

    async fn get_agent_mut(
        &self,
        name: &str,
    ) -> Option<tokio::sync::RwLockWriteGuard<'_, HashMap<String, Box<dyn Agent + Send + Sync>>>>
    {
        let guard = self.agents.write().await;
        if guard.contains_key(name) {
            Some(guard)
        } else {
            None
        }
    }

    async fn get_config(&self, name: &str) -> Option<Config> {
        self.configs.read().await.get(name).cloned()
    }

    async fn list_agents(&self) -> Result<(), Box<dyn std::error::Error>> {
        let agents = self.agents.read().await;
        println!("Active agents:");
        for (name, _) in agents.iter() {
            println!("- {}", name);
        }
        Ok(())
    }
}

async fn run_mcp_ping(config_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    use kowalski_cli::config::load_mcp_config_from_file;
    use std::path::Path;

    let path = config_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new("config.toml").to_path_buf());
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

    for server in &mcp.servers {
        print!(
            "  {} <{}> [{}] ... ",
            server.name,
            server.url,
            match server.transport {
                kowalski_core::config::McpTransport::Http => "http",
                kowalski_core::config::McpTransport::Sse => "sse",
            }
        );
        io::stdout().flush()?;
        match kowalski_core::mcp::McpClient::connect_server(server).await {
            Ok(client) => match client.list_tools().await {
                Ok(tools) => println!("OK — {} tool(s)", tools.len()),
                Err(e) => println!("partial — tools/list: {}", e),
            },
            Err(e) => println!("FAILED — {}", e),
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();
    let manager = AgentManager::new();

    let mut active_agent_name = None;

    if let Some(config_path) = &cli.config {
        match manager.create_agent_from_config(config_path).await {
            Ok(name) => active_agent_name = Some(name),
            Err(e) => {
                eprintln!("Error loading config: {}", e);
                return Err(e);
            }
        }
    }

    if cli.interactive {
        println!("Starting Kowalski in interactive mode...");
        let agent_name = active_agent_name.unwrap_or_else(|| {
            // Fallback to default if no config provided
            "default".to_string()
        });

        if manager.get_agent_mut(&agent_name).await.is_none() {
            manager
                .create_agent(agent_name.clone(), "web", None, None)
                .await?;
        }

        let mut agents_guard = manager.get_agent_mut(&agent_name).await.unwrap();
        if let Some(agent) = agents_guard.remove(&agent_name) {
            let mut session = kowalski_cli::interactive::InteractiveSession::new(agent, "llama3");
            session.run().await?;
            return Ok(());
        }
    }

    match cli.command {
        Some(Commands::Create {
            agent_type,
            prompt,
            temperature,
            name,
            config,
        }) => {
            if let Some(config_path) = config {
                manager.create_agent_from_config(&config_path).await?;
            } else {
                let name = name.unwrap_or_else(|| format!("{}-agent", agent_type));
                manager
                    .create_agent(name, &agent_type, prompt.as_deref(), temperature)
                    .await?;
            }
        }
        Some(Commands::Chat { agent, .. }) => {
            let agents_guard = manager.get_agent_mut(&agent).await;
            if let Some(mut agents_guard) = agents_guard {
                if let Some(agent_ref) = agents_guard.get_mut(&agent) {
                    let config = manager
                        .get_config(&agent)
                        .await
                        .unwrap_or_else(Config::default);
                    let conv_id = agent_ref.start_conversation(&config.ollama.model);
                    println!(
                        "Chat session started with agent '{}'. Type /bye to end chat.",
                        agent
                    );
                    println!("Model in use: {}", config.ollama.model);
                    // Print registered tools
                    let tools = agent_ref.list_tools().await;
                    if !tools.is_empty() {
                        info!("Registered tools:");
                        for (name, desc) in tools {
                            info!("  - {}: {}", name, desc);
                        }
                    } else {
                        info!("No tools registered or tool listing not available.");
                    }

                    chat_loop(agent_ref, conv_id).await?;
                } else {
                    println!("Agent '{}' not found.", agent);
                }
            } else {
                println!("Agent '{}' not found.", agent);
            }
        }
        Some(Commands::List) => list_agents()?,
        Some(Commands::Agents) => manager.list_agents().await?,
        Some(Commands::Mcp { command }) => match command {
            McpCommands::Ping {
                config: config_path,
            } => {
                run_mcp_ping(config_path.as_deref()).await?;
            }
        },
        Some(Commands::Consolidate { delete }) => {
            let config = Config::default();
            let episodic_path = &config.memory.episodic_path;
            let ollama_model = &config.ollama.model;

            // Create LLM provider for consolidation
            let llm_provider: std::sync::Arc<dyn kowalski_core::llm::LLMProvider> =
                std::sync::Arc::new(kowalski_core::llm::OllamaProvider::new(
                    &config.ollama.host,
                    config.ollama.port,
                ));

            let mut weaver =
                Consolidator::new(episodic_path, llm_provider, ollama_model)?;
            weaver.run(delete).await?;
            println!("Memory consolidation complete.");
        }
        None => {
            // Enter REPL mode if no subcommand is provided
            println!("Kowalski CLI Interactive Mode. Type 'help' for commands.");
            repl(manager).await?;
        }
    }
    Ok(())
}

async fn chat_loop(
    agent: &mut Box<dyn Agent + Send + Sync>,
    mut conv_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let agent_name = agent.name().to_lowercase();
    println!("Agent name: '{}'", agent_name);

    loop {
        print!("You: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input_trimmed = input.trim();

        if input_trimmed.eq_ignore_ascii_case("/bye") {
            println!("Goodbye!");
            break;
        }

        if input_trimmed.starts_with("/save") {
            let filename = input_trimmed.strip_prefix("/save").unwrap().trim();
            if filename.is_empty() {
                println!("Usage: /save <filename>");
            } else {
                match agent.export_conversation(&conv_id) {
                    Ok(json) => {
                        let _ = fs::create_dir_all("sessions");
                        let path = format!("sessions/{}.json", filename);
                        if let Err(e) = fs::write(&path, json) {
                            eprintln!("Failed to write session file: {}", e);
                        } else {
                            println!("Conversation saved to {}", path);
                        }
                    }
                    Err(e) => eprintln!("Failed to save conversation: {}", e),
                }
            }
            continue;
        }

        if input_trimmed.starts_with("/load") {
            let filename = input_trimmed.strip_prefix("/load").unwrap().trim();
            if filename.is_empty() {
                println!("Usage: /load <filename>");
            } else {
                let path = format!("sessions/{}.json", filename);
                match fs::read_to_string(&path) {
                    Ok(json) => match agent.import_conversation(&json) {
                        Ok(new_id) => {
                            conv_id = new_id;
                            println!("Conversation loaded. Current session ID: {}", conv_id);
                        }
                        Err(e) => eprintln!("Failed to import conversation: {}", e),
                    },
                    Err(e) => eprintln!("Failed to read session file: {}", e),
                }
            }
            continue;
        }

        // Always use tool-calling chat method
        info!("Using tool-calling chat method");
        match chat_with_tools(agent, &conv_id, &input).await {
            Ok(_) => {
                info!("Tool-calling chat completed successfully");
            }
            Err(e) => {
                eprintln!("Tool-calling chat failed: {}", e);
                // Optionally fallback to regular chat
                use_regular_chat(agent, &conv_id, &input).await?;
            }
        }
    }
    Ok(())
}

async fn chat_with_tools(
    agent: &mut Box<dyn Agent + Send + Sync>,
    conv_id: &str,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Use the agent's chat_with_tools method directly
    let _response = agent.chat_with_tools(conv_id, input).await?;
    // print!("{}", response); //this was already printed in chat_with_tools
    io::stdout().flush()?;
    Ok(())
}

async fn use_regular_chat(
    agent: &mut Box<dyn Agent + Send + Sync>,
    conv_id: &str,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Regular chat for non-web agents
    agent.add_message(conv_id, "user", input).await;
    let response = agent.chat_with_history(conv_id, input.trim(), None).await?;
    println!("{}", response);
    io::stdout().flush()?;
    println!();
    agent.add_message(conv_id, "assistant", input).await;
    Ok(())
}

fn list_agents() -> Result<(), Box<dyn std::error::Error>> {
    println!("Available agent types:");
    println!("- web: Web research and information retrieval");
    println!("- academic: Academic research and paper analysis");
    println!("- code: Code analysis, refactoring, and documentation");
    println!("- data: Data analysis and processing");
    Ok(())
}

async fn repl(manager: AgentManager) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        print!("kowalski> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        let mut parts = input.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        match cmd {
            "exit" | "quit" | "bye" | "/bye" => {
                println!("Exiting Kowalski CLI.");
                break;
            }
            "help" => {
                println!("Commands:");
                println!("  create <type> [--name <name>]: Create an agent");
                println!("  chat <name>: Chat with an agent");
                println!("  list: List available agent types");
                println!("  agents: List active agents");
                println!("  bye | /bye : Exit the CLI");
            }
            "create" => {
                let agent_type = parts.next();
                let name = parts.next();
                if let Some(agent_type) = agent_type {
                    let agent_name = match name {
                        Some(n) => n.to_string(),
                        None => format!("{}-agent", agent_type),
                    };
                    manager
                        .create_agent(agent_name.clone(), agent_type, None, None)
                        .await?;
                    println!("Agent created successfully: {}", agent_name);
                } else {
                    println!("Usage: create <type> [name]");
                }
            }
            "chat" => {
                let name = parts.next();
                if let Some(name) = name {
                    let agents_guard = manager.get_agent_mut(name).await;
                    if let Some(mut agents_guard) = agents_guard {
                        if let Some(agent_ref) = agents_guard.get_mut(name) {
                            let config = manager
                                .get_config(name)
                                .await
                                .unwrap_or_else(Config::default);
                            let conv_id = agent_ref.start_conversation(&config.ollama.model);
                            info!(
                                "Chat session started with agent '{}'. Type /bye to end chat.",
                                name
                            );
                            info!("[DEBUG] Model in use: {}", config.ollama.model);
                            // Print registered tools
                            let tools = agent_ref.list_tools().await;
                            if !tools.is_empty() {
                                info!("[DEBUG] Registered tools:");
                                for (name, desc) in tools {
                                    info!("  - {}: {}", name, desc);
                                }
                            } else {
                                info!("[DEBUG] No tools registered or tool listing not available.");
                            }

                            chat_loop(agent_ref, conv_id.clone()).await?;
                        } else {
                            println!("Agent '{}' not found.", name);
                        }
                    } else {
                        println!("Agent '{}' not found.", name);
                    }
                } else {
                    println!("Usage: chat <name>");
                }
            }
            "list" => {
                list_agents()?;
            }
            "agents" => {
                manager.list_agents().await?;
            }
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for a list of commands.",
                    cmd
                );
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn rule_based_tool_call(user_input: &str) -> Option<ToolCall> {
    let input = user_input.to_lowercase();
    if input.contains("list") && input.contains("directory") {
        if let Some(path) = input.split_whitespace().find(|w| w.starts_with('/')) {
            return Some(ToolCall {
                name: "fs_tool".to_string(),
                parameters: json!({ "task": "list_dir", "path": path }),
                reasoning: Some("Rule-based: user asked to list a directory".to_string()),
            });
        }
    }
    if input.contains("first 10 lines") && input.contains(".csv") {
        if let Some(path) = input.split_whitespace().find(|w| w.ends_with(".csv")) {
            return Some(ToolCall {
                name: "fs_tool".to_string(),
                parameters: json!({ "task": "get_file_first_lines", "path": path, "num_lines": 10 }),
                reasoning: Some("Rule-based: user asked for first 10 lines of a CSV".to_string()),
            });
        }
    }
    // Add more rules as needed...
    None
}
