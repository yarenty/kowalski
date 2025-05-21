use env_logger;
use kowalski_core::{
    agent::Agent,
    role::{Audience, Preset, Role},
};
use kowalski_web_agent::{agent::WebAgent, config::Config};
use log::info;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::load()?;
    let mut web_agent = WebAgent::new(config)?;

    // Start a conversation
    info!("🤖 Starting web agent...");
    let conversation_id = web_agent.start_conversation("llama2");
    info!("Web Agent Conversation ID: {}", conversation_id);

    // Perform a web search
    let query = "Latest developments in Rust programming";
    println!("\n🔍 Searching: {}", query);
    let search_results = web_agent.search(query).await?;

    // Process search results
    for result in &search_results {
        web_agent
            .add_message(
                &conversation_id,
                "search",
                format!("{} : {}", result.title, result.snippet).as_str(),
            )
            .await;

        println!("\n📑 Result:");
        println!("Title: {}", result.title);
        println!("URL: {}", result.url);
        println!("Snippet: {}", result.snippet);
    }

    // Add search query to conversation
    web_agent
        .add_message(
            &conversation_id,
            "user",
            format!("Search for {} and provide a summary", query).as_str(),
        )
        .await;

    // Process the first search result in detail
    if let Some(first_result) = search_results.first() {
        println!("\n🌐 Processing first result: {}", first_result.url);
        let page = web_agent.fetch_page(&first_result.url).await?;

        // Add page content to conversation
        web_agent
            .add_message(
                &conversation_id,
                "search",
                format!("Full page: {} : {}", page.title, page.content).as_str(),
            )
            .await;

        // Generate a simplified summary
        let role = Role::translator(Some(Audience::Family), Some(Preset::Simplify));
        println!("\n📝 Generating summary...");

        let mut response = web_agent
            .chat_with_history(&conversation_id, "Provide a simple summary", Some(role))
            .await?;

        // Process the streaming response
        let mut buffer = String::new();
        while let Some(chunk) = response.chunk().await? {
            match web_agent
                .process_stream_response(&conversation_id, &chunk)
                .await
            {
                Ok(Some(message)) => {
                    // Print the content if it exists
                    if !message.content.is_empty() {
                        print!("{}", message.content);
                        io::stdout().flush()?;
                        buffer.push_str(&message.content);
                    }

                    // Handle tool calls if they exist
                    if let Some(tool_calls) = &message.tool_calls {
                        for tool_call in tool_calls {
                            print!("\n[Tool Call] {}(", tool_call.function.name);
                            for (key, value) in &tool_call.function.arguments {
                                print!("{}: {}, ", key, value);
                            }
                            println!(")");
                            io::stdout().flush()?;
                        }
                    }
                }
                Ok(None) => {
                    web_agent
                        .add_message(&conversation_id, "assistant", &buffer)
                        .await;
                    println!("\n✅ Summary complete!\n");
                    break;
                }
                Err(e) => {
                    eprintln!("\n❌ Error processing stream: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
