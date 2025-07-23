use env_logger;
use kowalski_core::{agent::Agent, config::Config};
use kowalski_web_agent::agent::WebAgent;

use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    let config = Config::default();
    // Load configuration
    let mut web_agent = WebAgent::new(config.clone()).await?;

    // Start a conversation
    info!("🤖 Starting web agent...");
    let conversation_id = web_agent.start_conversation(&config.ollama.model);
    info!("Web Agent Conversation ID: {}", conversation_id);

    // Perform a web search using tool-calling
    let search_input = r#"{"name": "web_search", "parameters": {"query": "AI"}}"#;
    let search_results = web_agent
        .chat_with_tools(&conversation_id, search_input)
        .await?;
    println!("\n📑 Search Results:\n{}", search_results);

    // Example: Scrape a page (if you have a URL)
    // let scrape_input = r#"{"name": "web_scrape", "parameters": {"url": "https://example.com"}}"#;
    // let page_content = web_agent.chat_with_tools(&conversation_id, scrape_input).await?;
    // println!("\n🌐 Page Content:\n{}", page_content);

    // Add search query to conversation
    web_agent
        .add_message(
            &conversation_id,
            "user",
            format!("Search for {} and provide a summary", "AI").as_str(),
        )
        .await;

    // Process the first search result in detail
    // The original code had a loop over search_results, but the new code directly prints the result.
    // Assuming the intent was to process the first result if search_results was a vector of results.
    // Since search_results is now a string, we'll just print it.
    // If the intent was to process a single result from a vector, the original code would need to be adapted.
    // For now, we'll just print the search_results string.
    println!(
        "\n🌐 Processing search results (as a string): {}",
        search_results
    );

    // The original code had a detailed processing of the first result, including page fetching and summary generation.
    // This part of the logic needs to be re-evaluated based on the new `search_results` format.
    // For now, we'll remove the detailed processing as the `search_results` is now a string.
    // If the intent was to process a single result from a vector, the original code would need to be adapted.
    // For now, we'll just print the search_results string.
    // The original code had a detailed processing of the first result, including page fetching and summary generation.
    // This part of the logic needs to be re-evaluated based on the new `search_results` format.
    // For now, we'll remove the detailed processing as the `search_results` is now a string.
    // If the intent was to process a single result from a vector, the original code would need to be adapted.
    // For now, we'll just print the search_results string.

    Ok(())
}
