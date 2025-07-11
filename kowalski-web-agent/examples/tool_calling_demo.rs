use kowalski_core::config::Config;
use kowalski_web_agent::WebAgent;
use kowalski_core::agent::Agent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kowalski Web Agent Tool-Calling Demo");
    println!("=====================================");

    // Create a web agent
    let config = Config::default();
    let mut agent = WebAgent::new(config).await?;

    // Start a conversation
    let conv_id = agent.start_conversation("llama3.2");
    println!("Conversation started with ID: {}", conv_id);

    // Example queries that should trigger tool usage
    let queries = vec![
        "What's the latest news about AI?",
        "Search for information about Rust programming language",
        "Find recent developments in machine learning",
    ];

    for query in queries {
        println!("\n--- Testing Query: {} ---", query);

        // Use the enhanced tool-calling method
        match agent.chat_with_tools(&conv_id, query).await {
            Ok(response) => {
                println!("Final Response: {}", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    println!("\nDemo completed!");
    Ok(())
}
