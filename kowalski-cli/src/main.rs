use clap::Parser;
use kowalski_core::agent::Agent;
use kowalski_core::config::Config;
use kowalski_web_agent::WebAgent;
use kowalski_academic_agent::AcademicAgent;
use kowalski_code_agent::CodeAgent;
use kowalski_data_agent::DataAgent;
use kowalski_agent_template::templates::general::GeneralTemplate;
use kowalski_agent_template::builder::AgentBuilder;
use kowalski_core::error::KowalskiError;
use std::io::{self, Write};
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Create a new agent
    Create {
        /// Agent type (web, academic, code, data)
        agent_type: String,
        /// Optional system prompt
        #[arg(short, long)]
        prompt: Option<String>,
        /// Optional temperature
        #[arg(short, long)]
        temperature: Option<f32>,
        /// Optional agent name
        #[arg(short, long)]
        name: Option<String>,
        /// Optional configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Chat with an agent
    Chat {
        /// Agent name or type
        agent: String,
        /// Optional system prompt
        #[arg(short, long)]
        prompt: Option<String>,
        /// Optional temperature
        #[arg(short, long)]
        temperature: Option<f32>,
        /// Optional model
        #[arg(short, long)]
        model: Option<String>,
    },
    /// List available agent types
    List,
    /// List active agents
    Agents,
    /// Create a federation of agents
    Federation {
        /// Agent names to include in federation
        agents: Vec<String>,
        /// Optional system prompt for federation
        #[arg(short, long)]
        prompt: Option<String>,
        /// Optional temperature
        #[arg(short, long)]
        temperature: Option<f32>,
        /// Optional name for federation
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Show documentation for an agent or tool
    Doc {
        /// Agent or tool name
        target: String,
    },
    /// Delete an agent
    Delete {
        /// Agent name to delete
        name: String,
    },
    /// Save agent configuration
    Save {
        /// Agent name
        name: String,
        /// Output file path
        path: String,
    },
    /// Load agent configuration
    Load {
        /// Input file path
        path: String,
    },
    /// Export agent state
    Export {
        /// Agent name
        name: String,
        /// Output file path
        path: String,
    },
    /// Import agent state
    Import {
        /// Input file path
        path: String,
    },
    /// Show agent status
    Status {
        /// Agent name
        name: String,
    },
    /// List available models
    Models,
    /// Set default model
    Model {
        /// Model name
        name: String,
    },
}

#[derive(Debug)]
struct AgentManager {
    agents: Arc<RwLock<HashMap<String, AgentBuilder>>>,
    configs: Arc<RwLock<HashMap<String, Config>>>,
    models: Arc<RwLock<HashMap<String, String>>>,
}

impl AgentManager {
    fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_agent(
        &self,
        name: String,
        agent_type: &str,
        prompt: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        let mut builder = match agent_type {
            "web" => WebAgent::new(config).await?.template,
            "academic" => AcademicAgent::new(config).await?.template,
            "code" => CodeAgent::new(config).await?.template,
            "data" => DataAgent::new(config).await?.template,
            _ => {
                eprintln!("Unknown agent type: {}", agent_type);
                return Ok(());
            }
        };

        if let Some(prompt) = prompt {
            builder = builder.with_system_prompt(prompt);
        }

        if let Some(temp) = temperature {
            builder = builder.with_temperature(temp);
        }

        self.agents.write().await.insert(name, builder);
        println!("Agent created successfully: {}", name);
        Ok(())
    }

    async fn get_agent(&self, name: &str) -> Option<AgentBuilder> {
        self.agents.read().await.get(name).cloned()
    }

    async fn list_agents(&self) -> Result<(), Box<dyn std::error::Error>> {
        let agents = self.agents.read().await;
        println!("Active agents:");
        for (name, _) in agents.iter() {
            println!("- {}", name);
        }
        Ok(())
    }

    async fn create_federation(
        &self,
        agent_names: Vec<String>,
        prompt: Option<&str>,
        temperature: Option<f32>,
    ) -> Result<AgentBuilder, Box<dyn std::error::Error>> {
        let mut federation_builder = GeneralTemplate::create_default_agent().await?;

        if let Some(prompt) = prompt {
            federation_builder = federation_builder.with_system_prompt(prompt);
        }

        if let Some(temp) = temperature {
            federation_builder = federation_builder.with_temperature(temp);
        }

        // Add all agents to federation
        for name in agent_names {
            if let Some(agent) = self.get_agent(&name).await {
                federation_builder = federation_builder.with_tool(agent.build().await?);
            } else {
                eprintln!("Warning: Agent {} not found", name);
            }
        }

        Ok(federation_builder)
    }

    async fn set_default_model(&self, model: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.models.write().await.insert("default".to_string(), model.to_string());
        println!("Default model set to: {}", model);
        Ok(())
    }

    async fn get_default_model(&self) -> Option<String> {
        self.models.read().await.get("default").cloned()
    }

    async fn list_models(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Available models:");
        for (name, _) in self.models.read().await.iter() {
            println!("- {}", name);
        }
        Ok(())
    }

    async fn export_agent(&self, name: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(builder) = self.agents.read().await.get(name) {
            let state = serde_json::to_string_pretty(&builder)?;
            std::fs::write(path, state)?;
            println!("Agent state exported to: {}", path);
        } else {
            eprintln!("Agent {} not found", name);
        }
        Ok(())
    }

    async fn import_agent(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let state = std::fs::read_to_string(path)?;
        let builder: AgentBuilder = serde_json::from_str(&state)?;
        let name = format!("imported-agent-{}", chrono::Utc::now().timestamp());
        self.agents.write().await.insert(name.clone(), builder);
        println!("Agent imported successfully: {}", name);
        Ok(())
    }

    async fn show_agent_status(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(builder) = self.agents.read().await.get(name) {
            let agent = builder.build().await?;
            println!("Agent Status:");
            println!("Name: {}", name);
            println!("Type: {}", agent.name());
            println!("Description: {}", agent.description());
            println!("System Prompt: {}", agent.system_prompt());
            println!("Temperature: {}", agent.temperature());
            println!("Tools:");
            for tool in agent.tools() {
                println!("- {}", tool.name());
                println!("  Description: {}", tool.description());
            }
        } else {
            eprintln!("Agent {} not found", name);
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let manager = AgentManager::new();

    match cli.command {
        Commands::Create { agent_type, prompt, temperature, name, config } => {
            let name = name.unwrap_or_else(|| format!("{}-agent", agent_type));
            manager.create_agent(name, &agent_type, prompt.as_deref(), temperature).await?
        }
        Commands::Chat { agent, prompt, temperature, model } => {
            let mut builder = match agent.parse::<usize>() {
                Ok(idx) => manager.agents.read().await.values().nth(idx).cloned(),
                Err(_) => manager.get_agent(&agent).await,
            }
            .ok_or_else(|| format!("Agent {} not found", agent))?;

            if let Some(prompt) = prompt {
                builder = builder.with_system_prompt(prompt);
            }

            if let Some(temp) = temperature {
                builder = builder.with_temperature(temp);
            }

            if let Some(model) = model {
                builder = builder.with_model(model);
            }

            let agent = builder.build().await?;
            let conv_id = agent.start_conversation("default").await;

            println!("Chat session started with agent");
            println!("----------------------------------------");

            chat_loop(agent, conv_id).await
        }
        Commands::List => {
            list_agents()?
        }
        Commands::Agents => {
            manager.list_agents().await?
        }
        Commands::Federation { agents, prompt, temperature, name } => {
            let federation_builder = manager.create_federation(agents, prompt.as_deref(), temperature).await?;
            let agent = federation_builder.build().await?;
            let conv_id = agent.start_conversation("default").await;

            println!("Federation session started with agents: {:?}", agents);
            println!("----------------------------------------");

            chat_loop(agent, conv_id).await
        }
        Commands::Doc { target } => {
            // Show documentation for agent or tool
            println!("Documentation for {}: Not implemented yet", target);
        }
        Commands::Delete { name } => {
            // Delete agent
            println!("Delete agent {}: Not implemented yet", name);
        }
        Commands::Save { name, path } => {
            // Save agent configuration
            println!("Save agent configuration {}: Not implemented yet", name);
        }
        Commands::Load { path } => {
            // Load agent configuration
            println!("Load agent configuration: Not implemented yet");
        }
        Commands::Export { name, path } => {
            manager.export_agent(&name, &path).await?
        }
        Commands::Import { path } => {
            manager.import_agent(&path).await?
        }
        Commands::Status { name } => {
            manager.show_agent_status(&name).await?
        }
        Commands::Models => {
            manager.list_models().await?
        }
        Commands::Model { name } => {
            manager.set_default_model(&name).await?
        }
    }

    Ok(())
}

async fn chat_loop(agent: Agent, conv_id: String) -> Result<(), Box<dyn std::error::Error>> {
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
        let mut stream = agent.execute_task(ToolInput::new(
            "chat".to_string(),
            input.trim().to_string(),
            json!({}),
        ))
        .await?
        .stream();

        while let Some(message) = stream.next().await {
            match message {
                Ok(message) => {
                    if !message.content.is_empty() {
                        print!("{}", message.content);
                        io::stdout().flush()?;
                        buffer.push_str(&message.content);
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
