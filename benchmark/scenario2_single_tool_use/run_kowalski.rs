use kowalski_core::agent::{Agent, BaseAgent};
use kowalski_core::config::Config;
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = Config::default();
    let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await?;
    let conversation_id = agent.start_conversation("llama3.2");

    // NOTE: For this to work, you need to have a tool named "weather_tool"
    // registered with the agent and its execute_tool method implemented.
    // This is a placeholder for the actual tool call logic.

    let start_time = Instant::now();
    let response = agent.chat_with_tools(&conversation_id, "What's the weather in London?").await?;
    let elapsed = start_time.elapsed();

    println!("Kowalski (Single Tool Use) - Response: {}", response);
    println!("Kowalski (Single Tool Use) - Time: {:?}", elapsed);

    Ok(())
}
