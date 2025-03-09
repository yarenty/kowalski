use kowalski::{
    agent::{Agent, ToolingAgent},
    config::Config,
};
use std::io::Write;
use log::info;
use env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::load()?;
    let mut agent = ToolingAgent::new(config)?;

    // Process a dynamic website
    let url = "https://twitter.com/rustlang";
    println!("🌐 Processing dynamic content from: {}", url);

    let page = agent.fetch_page(url).await?;
    println!("\n📑 Extracted Content:");
    println!("Title: {}", page.title);
    println!("\nContent Preview:");
    println!("{:.500}...", page.content);
    println!("\nMetadata:");
    for (key, value) in page.metadata {
        println!("  {}: {}", key, value);
    }

    Ok(())
} 