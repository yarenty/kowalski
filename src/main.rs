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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = config::Config::load()?;
    
    // Initialize model manager
    let model_manager = ModelManager::new(config.ollama.base_url.clone())?;
    let model_name = DEFAULT_MODEL.to_string();

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

    let msg = r#" Large Language Models (LLMs) have the potential to revolutionize data analytics  by simplifying tasks such as data discovery and SQL query synthesis through natural language interactions. This work serves as a pivotal first step toward the development of foundation models explicitly designed for data analytics applications. To propel this vision forward, we unveil a new data recipe for post-training LLMs, enhancing their comprehension of data management and empowering them to tackle complex real-world analytics tasks. Specifically, our innovative approach includes a scalable synthetic data generation method that enables the creation of a broad spectrum of topics centered on data representation and manipulation. Furthermore, we introduce two new tasks that seamlessly bridge tables and text. We show that such tasks can enhance models' understanding of schema creation and the nuanced translation between natural language and tabular data. Leveraging this data recipe, we post-train a new foundation model, named CoddLLM, based on Mistral-NeMo-12B. To assess the language understanding and reasoning capabilities of LLMs in the realm of data analytics, we contribute AnalyticsMMLU, a benchmark containing thousands of multiple-choice questions on databases, data analysis, and machine learning. Our focus on data discovery, has resulted in the contribution of three comprehensive benchmarks that address both database and data lake scenarios. CoddLLM not only excels in performance but also sets a new standard, achieving the highest average accuracy across eight datasets. It outperforms GPT-3.5-Turbo on AnalyticsMMLU, exceeding GPT-4o by 12.1% in table selection and showing an average improvement of 24.9% in Text-to-SQL compared to the base model."#;

    // Chat with history
    println!("\nChatting with history...");
    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
    let mut response = agent.stream_chat_with_history(&conversation_id, msg, Some(role)).await?;
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