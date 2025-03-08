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

use agent::{Agent, AcademicAgent, ToolingAgent};
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

    // Initialize agents
    let mut academic_agent = AcademicAgent::new(config.clone())?;
    let mut tooling_agent = ToolingAgent::new(config)?;

    // Example: Process a research paper
    println!("\nProcessing research paper...");
    let conversation_id = academic_agent.start_conversation(&model_name);
    println!("Academic Agent Conversation ID: {}", conversation_id);

    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
    let mut response = academic_agent
        .chat_with_history(
            &conversation_id,
            "/opt/research/2025/coddllm_2502.00329v1.pdf",
            Some(role),
        )
        .await?;

    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await? {
        match academic_agent.process_stream_response(&conversation_id, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(&content);
            }
            Ok(None) => {
                academic_agent.add_message(&conversation_id, "assistant", &buffer).await;
                println!("\n");
                break;
            }
            Err(e) => {
                eprintln!("\nError processing stream: {}", e);
                break;
            }
        }
    }

    // Example: Web search and processing
    println!("\nPerforming web search...");
    let conversation_id = tooling_agent.start_conversation(&model_name);
    println!("Tooling Agent Conversation ID: {}", conversation_id);

    let search_results = tooling_agent.search("Latest developments in Rust programming").await?;
    for result in &search_results {
        println!("Title: {}", result.title);
        println!("URL: {}", result.url);
        println!("Snippet: {}", result.snippet);
        println!();
    }

    // Process the first search result
    if let Some(first_result) = search_results.first() {
        println!("\nProcessing first search result...");
        let page = tooling_agent.fetch_page(&first_result.url).await?;
        
        let role = Role::translator(Some(Audience::Developer), Some(Preset::Summarize));
        let mut response = tooling_agent
            .chat_with_history(&conversation_id, &page.content, Some(role))
            .await?;

        let mut buffer = String::new();
        while let Some(chunk) = response.chunk().await? {
            match tooling_agent.process_stream_response(&conversation_id, &chunk).await {
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                    buffer.push_str(&content);
                }
                Ok(None) => {
                    tooling_agent.add_message(&conversation_id, "assistant", &buffer).await;
                    println!("\n");
                    break;
                }
                Err(e) => {
                    eprintln!("\nError processing stream: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
