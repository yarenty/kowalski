use clap::Parser;
use kowalski_academic_agent::AcademicAgent;
use kowalski_code_agent::CodeAgent;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
// use kowalski_data_agent::DataAgent;
use kowalski_web_agent::WebAgent;
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use futures::StreamExt;
use serde::Deserialize;

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
            _ => {
                eprintln!("Unknown agent type: {}", agent_type);
                return Ok(());
            }
        };
        self.agents.write().await.insert(name.clone(), agent);
        self.configs.write().await.insert(name, config);
        Ok(())
    }

    async fn get_agent_mut(&self, name: &str) -> Option<tokio::sync::RwLockWriteGuard<'_, HashMap<String, Box<dyn Agent + Send + Sync>>>> {
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

#[derive(Deserialize)]
struct ToolCall {
    tool: String,
    input: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let manager = AgentManager::new();

    match cli.command {
        Some(Commands::Create { agent_type, prompt, temperature, name, config }) => {
            let name = name.unwrap_or_else(|| format!("{}-agent", agent_type));
            manager.create_agent(name, &agent_type, prompt.as_deref(), temperature).await?
        }
        Some(Commands::Chat { agent, .. }) => {
            let mut agents_guard = manager.get_agent_mut(&agent).await;
            if let Some(mut agents_guard) = agents_guard {
                if let Some(agent_ref) = agents_guard.get_mut(&agent) {
                    let config = manager.get_config(&agent).await.unwrap_or_else(Config::default);
                    let conv_id = agent_ref.start_conversation(&config.ollama.model);
                    println!("Chat session started with agent '{}'. Type /bye to end chat.", agent);
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

async fn chat_loop(agent: &mut Box<dyn Agent + Send + Sync>, conv_id: String) -> Result<(), Box<dyn std::error::Error>> {
    let agent_name = agent.name().to_lowercase();
    let is_web_agent = agent_name.contains("web");
    loop {
        print!("You: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("/bye") {
            println!("Goodbye!");
            break;
        }
        agent.add_message(&conv_id, "user", &input).await;
        let mut buffer = String::new();
        let mut last_tool_result = None;
        // ReAct loop for web-agent
        loop {
            let response = agent
                .chat_with_history(&conv_id, input.trim(), None)
                .await?;
            let mut stream = response.bytes_stream();
            buffer.clear();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        if let Ok(Some(message)) = agent.process_stream_response(&conv_id, &bytes).await {
                            if !message.content.is_empty() {
                                print!("{}", message.content);
                                io::stdout().flush()?;
                                buffer.push_str(&message.content);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("\nError: {}", e);
                        break;
                    }
                }
            }
            println!("");
            // Try to parse buffer as a tool call
            if is_web_agent {
                if let Ok(tool_call) = serde_json::from_str::<ToolCall>(&buffer) {
                    // Execute the tool
                    let tool_result = match tool_call.tool.as_str() {
                        "web_search" => run_web_search(&tool_call.input).await,
                        "web_scrape" => run_web_scrape(&tool_call.input).await,
                        _ => Err(format!("Unknown tool: {}", tool_call.tool).into()),
                    };
                    match tool_result {
                        Ok(result) => {
                            let tool_result_str = format!("Tool result: {}", result);
                            agent.add_message(&conv_id, "tool", &tool_result_str).await;
                            last_tool_result = Some(tool_result_str);
                        }
                        Err(e) => {
                            let err_str = format!("Tool error: {}", e);
                            agent.add_message(&conv_id, "tool", &err_str).await;
                            println!("{}", err_str);
                            break;
                        }
                    }
                    // Continue loop: feed tool result as next user input
                    input = last_tool_result.clone().unwrap_or_default();
                    continue;
                }
            }
            // Not a tool call, print the answer and break
            agent.add_message(&conv_id, "assistant", &buffer).await;
            break;
        }
    }
    Ok(())
}

// Dummy implementations for tool execution (replace with real logic)
async fn run_web_search(query: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("[web_search executed for query: {}]", query))
}

async fn run_web_scrape(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("[web_scrape executed for url: {}]", url))
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
                    manager.create_agent(agent_name.clone(), agent_type, None, None).await?;
                    println!("Agent created successfully: {}", agent_name);
                } else {
                    println!("Usage: create <type> [name]");
                }
            }
            "chat" => {
                let name = parts.next();
                if let Some(name) = name {
                    let mut agents_guard = manager.get_agent_mut(name).await;
                    if let Some(mut agents_guard) = agents_guard {
                        if let Some(agent_ref) = agents_guard.get_mut(name) {
                            let config = manager.get_config(name).await.unwrap_or_else(Config::default);
                            let conv_id = agent_ref.start_conversation(&config.ollama.model);
                            println!("Chat session started with agent '{}'. Type /bye to end chat.", name);
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
                println!("Unknown command: {}. Type 'help' for a list of commands.", cmd);
            }
        }
    }
    Ok(())
}
