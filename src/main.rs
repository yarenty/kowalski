mod agent;

use agent::{Agent, Message, DEFAULT_MODEL};
use std::io::{self, Write};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = Agent::new(None)?;

    // List available models
    println!("Listing available models...");
    let models = agent.list_models().await?;
    for model in models.models {
        println!("Model: {}, Size: {} bytes, Modified: {}", 
            model.name, model.size, model.modified_at);
    }

    // Check if llama2:3.2 exists and pull it if needed
    if !agent.model_exists(DEFAULT_MODEL).await? {
        println!("Pulling model {}...", DEFAULT_MODEL);
        let mut stream = agent.pull_model(DEFAULT_MODEL).await?;
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

    // Chat example
    let messages = vec![Message {
        role: "user".to_string(),
        content: "why is the sky blue?".to_string(),
    }];

    println!("\nStarting chat with {}...", DEFAULT_MODEL);
    let mut stream = agent.stream_chat(DEFAULT_MODEL, messages).await?;
    let mut buffer = String::new();
    
    while let Some(chunk) = stream.chunk().await? {
        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
            let v: Value = serde_json::from_str(&text)?;
            if let Some(content) = v["message"]["content"].as_str() {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(content);
            }
        }
    }
    println!("\nChat complete!");

    Ok(())
}