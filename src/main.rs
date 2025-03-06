mod agent;
mod config;
mod audience;
mod preset;
mod role;
mod style;
mod conversation;
mod model;

use agent::Agent;
use std::io::{self, Write};
use serde_json::Value;
use audience::Audience;
use preset::Preset;
use role::Role;
use style::Style;
use model::{DEFAULT_MODEL, ModelManager};
use std::fs;

fn read_input_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    Ok(content)
}

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
        println!("Model: {}, Size: {} bytes, Modified: {}", 
            model.name, model.size, model.modified_at);
    }

    // Check if default model exists and pull it if needed
    if !model_manager.model_exists(&model_name).await? {
        println!("Pulling model {}...", model_name);
        let mut stream = model_manager.pull_model(&model_name).await?;
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
    let conversation_id = agent.start_conversation(&model_name);
    println!("Conversation ID: {}", conversation_id);

    // Read input from file
    let msg = read_input_file("input.txt")?;

    // Chat with history
    println!("\nChatting with history...");
    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
    let mut response = agent.stream_chat_with_history(&conversation_id, &msg, Some(role)).await?;
    let mut buffer = String::new();

    while let Some(chunk) = response.chunk().await? {
        match agent.process_stream_response(&conversation_id, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(&content);
            }
            Ok(None) => {
                // Stream is complete, final message has been added to conversation
                agent.add_message(&conversation_id, "assistant", &buffer).await;
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