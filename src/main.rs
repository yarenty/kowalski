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

use agent::Agent;
use model::ModelManager;
use role::{Audience, Preset, Role};
use serde_json::Value;
use std::fs;
use std::io::{self, Write};
use utils::{PaperCleaner, PdfReader};

/// Reads input from a file, because apparently typing is too mainstream.
/// "File reading is like opening presents - you never know what you're gonna get."
///
/// # Arguments
/// * `file_path` - The path to the file (which is probably too long and boring)
///
/// # Returns
/// * `Result<String, Box<dyn std::error::Error>>` - Either the file contents or an error that will make you question your career choices
fn read_input_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    if file_path.to_lowercase().ends_with(".pdf") {
        Ok(PdfReader::read_pdf(file_path)?)
    } else {
        Ok(fs::read_to_string(file_path)?)
    }
}

/// The main function that makes everything work (or at least tries to).
/// "Main functions are like first dates - they're exciting but usually end in disappointment."
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = config::Config::load()?;

    // Initialize model manager
    let model_manager = ModelManager::new(config.ollama.base_url.clone())?;
    let model_name = "michaelneale/deepseek-r1-goose";

    // List available models
    println!("Listing available models...");
    let models = model_manager.list_models().await?;
    for model in models.models {
        println!(
            "Model: {}, Size: {} bytes, Modified: {}",
            model.name, model.size, model.modified_at
        );
    }

    // Check if default model exists and pull it if needed
    if !model_manager.model_exists(model_name).await? {
        println!("Pulling model {}...", model_name);
        let mut stream = model_manager.pull_model(model_name).await?;
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

    // Initialize agent with config
    let mut agent = Agent::new(config)?;

    // Start a new conversation
    println!("\nStarting a new conversation...");
    let conversation_id = agent.start_conversation(model_name);
    println!("Conversation ID: {}", conversation_id);

    // Read input from file (supports both PDF and text files)
    let msg = read_input_file("/opt/research/2025/coddllm_2502.00329v1.pdf")?;

    println!("{}", &msg);

    println!(" Cleaning...");
    let msg = PaperCleaner::clean(&msg)?;

    println!("{}", &msg);
    // Chat with history
    println!("\nChatting with history...");
    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
    let mut response = agent
        .stream_chat_with_history(&conversation_id, &msg, Some(role))
        .await?;
    let mut buffer = String::new();

    while let Some(chunk) = response.chunk().await? {
        match agent
            .process_stream_response(&conversation_id, &chunk)
            .await
        {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(&content);
            }
            Ok(None) => {
                // Stream is complete, final message has been added to conversation
                agent
                    .add_message(&conversation_id, "assistant", &buffer)
                    .await;
                println!("\n");
                break;
            }
            Err(e) => {
                eprintln!("\nError processing stream: {}", e);
                break;
            }
        }
    }

    // Get conversation history
    if let Some(conversation) = agent.get_conversation(&conversation_id) {
        println!("\nConversation History:");
        for message in &conversation.messages {
            println!("{}: {}", message.role, message.content);
        }
    }

    Ok(())
}
