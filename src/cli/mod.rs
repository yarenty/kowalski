mod commands;
mod interactive;

pub use commands::{Cli, Commands, ModelCommands, ToolCommands};
pub use interactive::{academic_loop, chat_loop, tooling_loop};

use crate::agent::{AcademicAgent, Agent, GeneralAgent, ToolingAgent};
use crate::config::Config;
use crate::model::ModelManager;
use log::info;
use std::io::{self, Write};

/// Handles the CLI command execution
pub async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load()?;
    info!(
        "Loaded configuration with search provider: {}",
        config.search.provider
    );

    // Initialize model manager
    let model_manager = ModelManager::new(config.ollama.base_url.clone())?;

    match cli.command {
        Commands::Chat { message, model } => {
            let mut agent = GeneralAgent::new(config)?;
            let conv_id = agent.start_conversation(&model);
            chat_loop(agent, conv_id, &model, &message).await?;
        }

        Commands::Academic {
            file,
            model,
            format: _,
        } => {
            let mut agent = AcademicAgent::new(config)?;
            let conv_id = agent.start_conversation(&model);
            academic_loop(agent, conv_id, &model, &file).await?;
        }

        Commands::Model { command } => match command {
            ModelCommands::List => {
                let models = model_manager.list_models().await?;
                for model in models.models {
                    println!(
                        "Model: {}, Size: {} bytes, Modified: {}",
                        model.name, model.size, model.modified_at
                    );
                }
            }

            ModelCommands::Pull { name } => {
                println!("Pulling model {}...", name);
                let mut stream = model_manager.pull_model(&name).await?;
                while let Some(chunk) = stream.chunk().await? {
                    if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                        let v: serde_json::Value = serde_json::from_str(&text)?;
                        if let Some(status) = v["status"].as_str() {
                            print!("Status: {}\r", status);
                            io::stdout().flush()?;
                        }
                    }
                }
                println!("\nModel pulled successfully!");
            }

            ModelCommands::Remove { name } => {
                println!("Removing model {}...", name);
                model_manager.delete_model(&name).await?;
                println!("Model removed successfully!");
            }

            ModelCommands::Show { name } => {
                let models = model_manager.list_models().await?;
                if let Some(model) = models.models.iter().find(|m| m.name == name) {
                    println!("Model: {}", model.name);
                    println!("Size: {} bytes", model.size);
                    println!("Modified: {}", model.modified_at);
                } else {
                    println!("Model not found: {}", name);
                }
            }
        },

        Commands::Tool { command } => {
            let mut agent = ToolingAgent::new(config)?;
            let conv_id = agent.start_conversation(&command.model());

            match command {
                ToolCommands::Search {
                    query,
                    model,
                    limit,
                } => {
                    // Configure search parameters
                    agent.set_search_limit(limit);
                    tooling_loop(agent, conv_id, &model, &query, "Search").await?;
                }

                ToolCommands::Scrape {
                    url,
                    model,
                    follow_links,
                    max_depth,
                } => {
                    // Configure scraping parameters
                    agent.set_scrape_options(follow_links, max_depth);
                    tooling_loop(agent, conv_id, &model, &url, "Scrape").await?;
                }

                ToolCommands::Code {
                    path,
                    model,
                    language,
                } => {
                    // Configure code analysis parameters
                    if let Some(lang) = language {
                        agent.set_code_language(&lang);
                    }
                    tooling_loop(agent, conv_id, &model, &path.to_string_lossy(), "Code").await?;
                }
            }
        }
    }

    Ok(())
}
