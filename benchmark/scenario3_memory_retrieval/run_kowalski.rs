use kowalski_core::agent::{Agent, BaseAgent};
use kowalski_core::config::Config;
use kowalski_memory::{MemoryUnit, MemoryProvider};
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
    env_logger::init();

    let config = Config::default();
    let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await?;
    let conversation_id = agent.start_conversation("llama3.2");

    // Pre-populate semantic memory for testing
    // NOTE: This requires Qdrant to be running and the collection 'kowalski_memory' to exist.
    // The embedding vector size (4 in this example) must match Qdrant's collection config.
    let memory_unit1 = MemoryUnit {
        id: "proj_kowalski_desc".to_string(),
        timestamp: 1678886400, // Example timestamp
        content: "Kowalski is a high-performance, Rust-based framework for building AI agents.".to_string(),
        embedding: Some(vec![0.1, 0.2, 0.3, 0.4]), // Dummy embedding
    };
    let memory_unit2 = MemoryUnit {
        id: "rust_benefits".to_string(),
        timestamp: 1678886500,
        content: "Rust offers memory safety, concurrency without GIL, and high performance.".to_string(),
        embedding: Some(vec![0.5, 0.6, 0.7, 0.8]), // Dummy embedding
    };
    agent.semantic_memory.add(memory_unit1).await.expect("Failed to add memory unit 1");
    agent.semantic_memory.add(memory_unit2).await.expect("Failed to add memory unit 2");

    // Give Qdrant a moment to index (in a real scenario, this is handled by Qdrant itself)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let start_time = Instant::now();
    let response = agent.chat_with_history(&conversation_id, "Tell me about the project Kowalski and its benefits.", None).await?;
    let full_response = process_stream_response(response).await?;
    let elapsed = start_time.elapsed();

    println!("Kowalski (Memory Retrieval) - Response: {}", full_response);
    println!("Kowalski (Memory Retrieval) - Time: {:?}", elapsed);

    Ok(())
}
