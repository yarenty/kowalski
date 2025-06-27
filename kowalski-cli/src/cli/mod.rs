mod commands;
mod interactive;
#[cfg(feature = "data")]
mod data;

pub use academic::AcademicMode;
pub use commands::{Cli, Commands, ModelCommands, ToolCommands};
pub use interactive::InteractiveMode;
pub use web::WebMode;
#[cfg(feature = "data")]
pub use self::data::DataMode;

use kowalski_core::config::Config;
use kowalski_core::Agent;
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
            let mut web = WebMode::new(config).await?;
            interactive.chat_loop(&model).await?;
        }
      
        Commands::Model { command } => match command {
            ModelCommands::List => {
                interactive.list_models().await?;
            }
            ModelCommands::Pull { name } => {
                interactive.pull_model(&name).await?;
            }
            ModelCommands::Delete { name } => {
                interactive.delete_model(&name).await?;
            }
        },
    }

    Ok(())
}
