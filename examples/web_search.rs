use kowalski::{
    agent::{Agent, ToolingAgent},
    config::Config,
};
use env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::load()?;
    let mut agent = ToolingAgent::new(config)?;

    // Perform a search
    let query = "What's new in Rust 2024?";
    println!("🔍 Searching: {}", query);

    let results = agent.search(query).await?;
    for result in results {
        println!("\n📑 Result:");
        println!("Title: {}", result.title);
        println!("URL: {}", result.url);
        println!("Snippet: {}", result.snippet);
        println!();
    }

    Ok(())
} 