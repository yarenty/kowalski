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

    // List of static content sites to process
    let urls = vec![
        "https://www.rust-lang.org/",
        "https://doc.rust-lang.org/book/",
        "https://docs.rs/tokio/latest/tokio/",
    ];

    println!("🌐 Processing static content sites...\n");
    
    for url in urls {
        println!("Processing: {}", url);
        match agent.fetch_page(url).await {
            Ok(page) => {
                println!("Title: {}", page.title);
                println!("Content length: {} chars", page.content.len());
                println!("Metadata entries: {}", page.metadata.len());
                println!("\nContent preview: {:.200}...\n", page.content);
            }
            Err(e) => println!("Error processing {}: {}\n", url, e),
        }
    }

    Ok(())
} 