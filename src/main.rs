mod agent;
mod config;

use agent::{Agent, Message};
use std::io::{self, Write};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = Agent::new()?;
    let model_name = agent.get_default_model().to_string();

    // List available models
    println!("Listing available models...");
    let models = agent.list_models().await?;
    for model in models.models {
        println!("Model: {}, Size: {} bytes, Modified: {}", 
            model.name, model.size, model.modified_at);
    }

    // Check if default model exists and pull it if needed
    if !agent.model_exists(&model_name).await? {
        println!("Pulling model {}...", model_name);
        let mut stream = agent.pull_model(&model_name).await?;
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

    // Start a new conversation
    println!("\nStarting a new conversation...");
    let conversation_id = agent.start_conversation(&model_name);
    println!("Conversation ID: {}", conversation_id);

    // Chat with history
    let mut response = agent.stream_chat_with_history(&conversation_id, "Hello!").await?;
    let mut buffer = String::new();
    

    while let Some(chunk) = response.chunk().await? {
        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
            let v: Value = serde_json::from_str(&text)?;
            if let Some(content) = v["message"]["content"].as_str() {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(content);
            }
        }
    }


    // println!("Assistant: {:?}", response);

    // Get conversation history
    if let Some(conversation) = agent.get_conversation(&conversation_id) {
        println!("\nConversation History:");
        for message in &conversation.messages {
            println!("{}: {}", message.role, message.content);
        }
    }

    

    Ok(())
}