/// Main: The entry point of our AI-powered circus.
/// "Main functions are like orchestras - they make everything work together, but nobody notices until something goes wrong."
///
/// This is where the magic happens, or at least where we pretend it does.
/// Think of it as the conductor of our AI symphony, but with more error handling.
mod agent;
mod config;
mod conversation;
mod model;
mod role;
mod utils;
mod tools;
mod cli;

use agent::{Agent, AcademicAgent, GeneralAgent, ToolingAgent};
use model::ModelManager;
use role::{Audience, Preset, Role};
use serde_json::Value;
use std::fs;
use std::io::{self, Write};
use utils::PdfReader;
use env_logger;
use log::info;
use cli::{Cli, Commands, ModelCommands};

/// Reads input from a file, because apparently typing is too mainstream.
/// "File reading is like opening presents - you never know what you're gonna get."
///
/// # Arguments
/// * `file_path` - The path to the file (which is probably too long and boring)
///
/// # Returns
/// * `Result<String, Box<dyn std::error::Error>>` - Either the file contents or an error that will make you question your career choices
#[allow(dead_code)]
fn read_input_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if file_path.to_lowercase().ends_with(".pdf") {
        Ok(PdfReader::read_pdf_file(file_path)?)
    } else {
        Ok(fs::read_to_string(file_path)?)
    }
}

/// The main function that makes everything work (or at least tries to).
/// "Main functions are like first dates - they're exciting but usually end in disappointment."
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Load configuration
    let config = config::Config::load()?;
    info!("Loaded configuration with search provider: {}", config.search.provider);

    // Initialize model manager
    let model_manager = ModelManager::new(config.ollama.base_url.clone())?;

    match cli.command {
        Commands::Chat { message, model } => {
            // Create a general agent for chat
            let mut agent = GeneralAgent::new(config)?;
            let conv_id = agent.start_conversation(&model);

            // Process the chat message
            let mut response = agent
                .chat_with_history(&conv_id, &message, None)
                .await?;

            // Process streaming response
            while let Some(chunk) = response.chunk().await? {
                match agent.process_stream_response(&conv_id, &chunk).await {
                    Ok(Some(content)) => {
                        print!("{}", content);
                        io::stdout().flush()?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("\nError: {}", e);
                        break;
                    }
                }
            }
            println!();
        }

        Commands::Academic { file, model, format } => {
            // Create an academic agent
            let mut agent = AcademicAgent::new(config)?;
            let conv_id = agent.start_conversation(&model);

            // Read the file content
            let content = if file.extension().map_or(false, |ext| ext == "pdf") {
                PdfReader::read_pdf_file(&file.to_string_lossy())?
            } else {
                std::fs::read_to_string(&file)?
            };

            // Create a role for academic analysis
            let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

            // Process the content
            let mut response = agent
                .chat_with_history(&conv_id, &content, Some(role))
                .await?;

            // Process streaming response
            while let Some(chunk) = response.chunk().await? {
                match agent.process_stream_response(&conv_id, &chunk).await {
                    Ok(Some(content)) => {
                        print!("{}", content);
                        io::stdout().flush()?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("\nError: {}", e);
                        break;
                    }
                }
            }
            println!();
        }

        Commands::Model { command } => {
            match command {
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
                            let v: Value = serde_json::from_str(&text)?;
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
            }
        }
    }

    Ok(())
}
