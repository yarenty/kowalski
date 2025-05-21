mod commands;
mod interactive;
mod academic;
mod web;

pub use commands::{Cli, Commands, ModelCommands, ToolCommands};
pub use interactive::InteractiveMode;
pub use academic::AcademicMode;
pub use web::WebMode;

use kowalski_core::config::Config;
use log::info;

/// Handles the CLI command execution
pub async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::default();
    info!("Loaded configuration");

    // Initialize core interactive mode
    let interactive = InteractiveMode::new(config.clone())?;

    match cli.command {
        Commands::Chat { message, model } => {
            // Initialize web mode for chat (you can switch between web and academic based on config)
            let mut web = WebMode::new(config)?;
            web.chat_loop(&model).await?;
        }
        Commands::Academic { file, model, format } => {
            // Initialize academic mode for paper processing
            let mut academic = AcademicMode::new(config)?;
            academic.process_paper(&file, &model, &format).await?;
        }
        Commands::Model { command } => {
            match command {
                ModelCommands::List => {
                    interactive.list_models().await?;
                }
                ModelCommands::Pull { name } => {
                    interactive.pull_model(&name).await?;
                }
                ModelCommands::Delete { name } => {
                    interactive.delete_model(&name).await?;
                }
            }
        }
        Commands::Tool { command } => {
            // Initialize web mode for tool usage
            let mut web = WebMode::new(config)?;
            match command {
                ToolCommands::Search { query, model, limit } => {
                    let conv_id = web.agent.create_conversation(&model)?;
                    web.search(&conv_id, &query).await?;
                }
                ToolCommands::Browse { url, model, follow_links, max_depth } => {
                    let conv_id = web.agent.create_conversation(&model)?;
                    web.browse(&conv_id, &url).await?;
                }
            }
        }
    }

    Ok(())
} 