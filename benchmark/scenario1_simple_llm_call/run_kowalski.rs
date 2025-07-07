use kowalski_core::agent::{Agent, BaseAgent};
use kowalski_core::config::Config;
use kowalski_core::conversation::Message;
use tokio::time::Instant;
use futures::StreamExt;

async fn process_stream_response(mut response: reqwest::Response) -> Result<String, String> {
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                let text = String::from_utf8(bytes.to_vec())
                    .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                let stream_response: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("JSON parse error: {}", e))?;

                if let Some(message_val) = stream_response.get("message") {
                    if let Some(content) = message_val.get("content") {
                        buffer.push_str(content.as_str().unwrap_or(""));
                    }
                }
            }
            Err(e) => return Err(format!("Stream error: {}", e)),
        }
    }
    Ok(buffer)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize a basic logger for better output
    env_logger::init();

    let config = Config::default();
    let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await?;
    let conversation_id = agent.start_conversation("llama3.2");

    let start_time = Instant::now();
    let response = agent.chat_with_history(&conversation_id, "Tell me a short joke.", None).await?;
    let full_response = process_stream_response(response).await?;
    let elapsed = start_time.elapsed();

    println!("Kowalski (Simple LLM Call) - Response: {}", full_response);
    println!("Kowalski (Simple LLM Call) - Time: {:?}", elapsed);

    Ok(())
}
