use clap::Parser;
use futures::StreamExt;
use kowalski_academic_agent::AcademicAgent;
use kowalski_code_agent::CodeAgent;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_core::tools::ToolCall;
use kowalski_data_agent::DataAgent;
use kowalski_web_agent::WebAgent;
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
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

    async fn create_agent(
        &self,
        name: String,
        agent_type: &str,
        _prompt: Option<&str>,
        _temperature: Option<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        let agent: Box<dyn Agent + Send + Sync> = match agent_type {
            "web" => Box::new(WebAgent::new(config.clone()).await?),
            "academic" => Box::new(AcademicAgent::new(config.clone()).await?),
            "code" => Box::new(CodeAgent::new(config.clone()).await?),
            "data" => Box::new(DataAgent::new(config.clone()).await?),
            _ => {
                eprintln!("Unknown agent type: {}", agent_type);
                return Ok(());
            }
        };
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let manager = AgentManager::new();

    match cli.command {
        Some(Commands::Create {
            agent_type,
            prompt,
            temperature,
            name,
            config: _,
        }) => {
            let name = name.unwrap_or_else(|| format!("{}-agent", agent_type));
            manager
                .create_agent(name, &agent_type, prompt.as_deref(), temperature)
                .await?
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
                    println!("[DEBUG] Model in use: {}", config.ollama.model);
                    // --- DEBUG: Print registered tools if available ---
                    let any_agent = agent_ref.as_any();
                    if let Some(data_agent) = any_agent.downcast_ref::<DataAgent>() {
                        let tools = data_agent.list_tools().await;
                        println!("[DEBUG] Registered tools:");
                        for (name, desc) in tools {
                            println!("  - {}: {}", name, desc);
                        }
                    } else if let Some(academic_agent) = any_agent.downcast_ref::<AcademicAgent>() {
                        let tools = academic_agent.list_tools().await;
                        println!("[DEBUG] Registered tools:");
                        for (name, desc) in tools {
                            println!("  - {}: {}", name, desc);
                        }
                    } else if let Some(code_agent) = any_agent.downcast_ref::<CodeAgent>() {
                        let tools = code_agent.list_tools().await;
                        println!("[DEBUG] Registered tools:");
                        for (name, desc) in tools {
                            println!("  - {}: {}", name, desc);
                        }
                    } else if let Some(web_agent) = any_agent.downcast_ref::<WebAgent>() {
                        let tools = web_agent.list_tools().await;
                        println!("[DEBUG] Registered tools:");
                        for (name, desc) in tools {
                            println!("  - {}: {}", name, desc);
                        }
                    } else {
                        println!("[DEBUG] Tool listing not available for this agent type.");
                    }
                    // --- END DEBUG ---
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
    conv_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let agent_name = agent.name().to_lowercase();
    println!(
        "[DEBUG] Agent name: '{}', is_web_agent: {}",
        agent_name,
        agent_name.contains("web")
    );

    loop {
        print!("You: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("/bye") {
            println!("Goodbye!");
            break;
        }

        // Always use tool-calling chat method
        println!("[DEBUG] Using tool-calling chat method");
        match chat_with_tools(agent, &conv_id, &input).await {
            Ok(_) => {
                println!("[DEBUG] Tool-calling chat completed successfully");
            }
            Err(e) => {
                eprintln!("[DEBUG] Tool-calling chat failed: {}", e);
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
    let response = agent.chat_with_tools(conv_id, input).await?;
    print!("{}", response);
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
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                if let Ok(Some(message)) = agent.process_stream_response(conv_id, &bytes).await {
                    if !message.content.is_empty() {
                        print!("{}", message.content);
                        io::stdout().flush()?;
                    }
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }
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
            "exit" | "quit" => {
                println!("Exiting Kowalski CLI.");
                break;
            }
            "help" => {
                println!("Commands:");
                println!("  create <type> [--name <name>]: Create an agent");
                println!("  chat <name>: Chat with an agent");
                println!("  list: List available agent types");
                println!("  agents: List active agents");
                println!("  exit | quit: Exit the CLI");
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
                            println!(
                                "Chat session started with agent '{}'. Type /bye to end chat.",
                                name
                            );
                            println!("[DEBUG] Model in use: {}", config.ollama.model);
                            // --- DEBUG: Print registered tools if available ---
                            let any_agent = agent_ref.as_any();
                            if let Some(data_agent) = any_agent.downcast_ref::<DataAgent>() {
                                let tools = data_agent.list_tools().await;
                                println!("[DEBUG] Registered tools:");
                                for (name, desc) in tools {
                                    println!("  - {}: {}", name, desc);
                                }
                            } else if let Some(academic_agent) =
                                any_agent.downcast_ref::<AcademicAgent>()
                            {
                                let tools = academic_agent.list_tools().await;
                                println!("[DEBUG] Registered tools:");
                                for (name, desc) in tools {
                                    println!("  - {}: {}", name, desc);
                                }
                            } else if let Some(code_agent) = any_agent.downcast_ref::<CodeAgent>() {
                                let tools = code_agent.list_tools().await;
                                println!("[DEBUG] Registered tools:");
                                for (name, desc) in tools {
                                    println!("  - {}: {}", name, desc);
                                }
                            } else if let Some(web_agent) = any_agent.downcast_ref::<WebAgent>() {
                                let tools = web_agent.list_tools().await;
                                println!("[DEBUG] Registered tools:");
                                for (name, desc) in tools {
                                    println!("  - {}: {}", name, desc);
                                }
                            } else {
                                println!("[DEBUG] Tool listing not available for this agent type.");
                            }
                            // --- END DEBUG ---
                            chat_loop(agent_ref, conv_id).await?;
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
