use futures::StreamExt;
use kowalski_core::Agent;
use kowalski_core::BaseAgent;
use kowalski_core::config::Config;
use kowalski_core::error::KowalskiError;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional, but good practice)
    kowalski_core::logging::init();

    let config = Config::default();
    let mut agent = BaseAgent::new(
        config.clone(),
        "Simple Chat Agent",
        "A basic agent for interactive chat",
    )
    .await?;
    let conv_id = agent.start_conversation(&config.ollama.model);

    println!(
        "Chat session started with model: {} (type '/bye' to exit)",
        config.ollama.model
    );
    println!("----------------------------------------");

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "/bye" {
            println!("Goodbye!");
            break;
        }

        if input.is_empty() {
            continue;
        }

        let mut response = agent.chat_with_history(&conv_id, input, None).await?;

        print!("Kowalski: ");
        let mut buffer = String::new();
        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(&conv_id, &chunk).await {
                Ok(Some(message)) => {
                    if !message.content.is_empty() {
                        print!("{}", message.content);
                        io::stdout().flush()?;
                        buffer.push_str(&message.content);
                    }
                }
                Ok(None) => {
                    println!("\n");
                    agent.add_message(&conv_id, "assistant", &buffer).await;
                    break;
                }
                Err(e) => {
                    eprintln!("\nError: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}
