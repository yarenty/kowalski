use kowalski::{Config, UnifiedAgent, Agent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let mut agent = UnifiedAgent::new(config)?;
    let conv_id = agent.start_conversation("llama2");

    // The model will automatically decide when to use tools
    let response = agent.chat_with_history(
        &conv_id,
        "What are the latest Rust features? Please search and summarize.",
        None
    ).await?;

    // Process the response stream
    while let Some(chunk) = response.chunk().await? {
        if let Some(content) = agent.process_stream_response(&conv_id, &chunk).await? {
            println!("{}", content);
        }
    }

    Ok(())
}