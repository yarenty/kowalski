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

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
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
}

impl AgentManager {
    fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
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
            "web" => Box::new(WebAgent::new(config).await?),
            "academic" => Box::new(AcademicAgent::new(config).await?),
            "code" => Box::new(CodeAgent::new(config).await?),
            _ => {
                eprintln!("Unknown agent type: {}", agent_type);
                return Ok(());
            }
        };
        self.agents.write().await.insert(name, agent);
        println!("Agent created successfully: {}", agent_type);
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
        Commands::Create {
            agent_type,
            prompt,
            temperature,
            name,
            config,
        } => {
            let name = name.unwrap_or_else(|| format!("{}-agent", agent_type));
            manager
                .create_agent(name, &agent_type, prompt.as_deref(), temperature)
                .await?
        }
        Commands::Chat {
            agent,
            ..
        } => {
            let mut agents_guard = manager.get_agent_mut(&agent).await.ok_or_else(|| format!("Agent {} not found", agent))?;
            let agent_ref = agents_guard.get_mut(&agent).ok_or_else(|| format!("Agent {} not found", agent))?;
            let conv_id = agent_ref.start_conversation("default");
            println!("Chat session started with agent");
            println!("----------------------------------------");
            chat_loop(agent_ref, conv_id).await?;
        }
        Commands::List => list_agents()?,
        Commands::Agents => manager.list_agents().await?,
    }

    Ok(())
}

async fn chat_loop(agent: &mut Box<dyn Agent + Send + Sync>, conv_id: String) -> Result<(), Box<dyn std::error::Error>> {
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
        let response = agent
            .chat_with_history(&conv_id, input.trim(), None)
            .await?;
        let mut stream = response.bytes_stream();
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
        println!("\n");
        agent.add_message(&conv_id, "assistant", &buffer).await;
    }
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
