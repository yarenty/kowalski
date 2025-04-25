use env_logger;
use kowalski::model::ModelManager;
use log::info;
use serde_json::Value;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Initialize model manager with default Ollama URL
    let model_manager = ModelManager::new("http://localhost:11434".to_string())?;
    let model_name = "llama2"; // Using llama2 as it's typically quicker

    // List available models
    info!("📋 Listing available models...");
    let models = model_manager.list_models().await?;
    println!("\nAvailable Models:");
    println!("----------------");
    for model in models.models {
        println!(
            "• {}\n  Size: {} bytes\n  Modified: {}\n",
            model.name, model.size, model.modified_at
        );
    }

    // Check if model exists and pull if needed
    if !model_manager.model_exists(&model_name).await? {
        println!("🔄 Pulling model {}...", model_name);
        let mut stream = model_manager.pull_model(&model_name).await?;

        print!("\rProgress: ");
        io::stdout().flush()?;

        while let Some(chunk) = stream.chunk().await? {
            if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                let v: Value = serde_json::from_str(&text)?;
                if let Some(status) = v["status"].as_str() {
                    print!("\rStatus: {}", status);
                    io::stdout().flush()?;
                }
            }
        }
        println!("\n✅ Model pulled successfully!");
    } else {
        println!("✅ Model {} is already available", model_name);
    }

    Ok(())
}
