use futures::StreamExt;
use kowalski_core::agent::{Agent, BaseAgent};
use kowalski_core::config::Config;
use kowalski_memory::{MemoryProvider, MemoryUnit};
use tokio::time::Instant;
use reqwest;
use serde_json;

async fn process_stream_response(mut response: reqwest::Response) -> Result<String, String> {
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                let text = String::from_utf8(bytes.to_vec())
                    .map_err(|e| format!("Invalid UTF-8: {}", e))?;
                let stream_response: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?;

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

async fn get_ollama_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": "llama3.2",
            "prompt": text
        }))
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    let embedding = json["embedding"]
        .as_array()
        .ok_or("No embedding in response")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();
    Ok(embedding)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = Config::default();
    let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await?;
    let conversation_id = agent.start_conversation("llama3.2");

    // Pre-populate semantic memory for testing
    // NOTE: This requires Qdrant to be running and the collection 'kowalski_memory' to exist.
    // The embedding vector size must match Qdrant's collection config.
    let content1 = "Kowalski is a high-performance, Rust-based framework for building AI agents.";
    let content2 = "Rust offers memory safety, concurrency without GIL, and high performance.";
    let embedding1 = get_ollama_embedding(content1).await?;
    let embedding2 = get_ollama_embedding(content2).await?;
    let memory_unit1 = MemoryUnit {
        id: "proj_kowalski_desc".to_string(),
        timestamp: 1678886400, // Example timestamp
        content: content1.to_string(),
        embedding: Some(embedding1),
    };
    let memory_unit2 = MemoryUnit {
        id: "rust_benefits".to_string(),
        timestamp: 1678886500,
        content: content2.to_string(),
        embedding: Some(embedding2),
    };
    agent
        .semantic_memory
        .lock()
        .await
        .add(memory_unit1)
        .await
        .expect("Failed to add memory unit 1");
    agent
        .semantic_memory
        .lock()
        .await
        .add(memory_unit2)
        .await
        .expect("Failed to add memory unit 2");

    // Give Qdrant a moment to index (in a real scenario, this is handled by Qdrant itself)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let start_time = Instant::now();
    let response = agent
        .chat_with_history(
            &conversation_id,
            "Tell me about the project Kowalski and its benefits.",
            None,
        )
        .await?;
    let full_response = process_stream_response(response).await?;
    let elapsed = start_time.elapsed();

    println!("Kowalski (Memory Retrieval) - Response: {}", full_response);
    println!("Kowalski (Memory Retrieval) - Time: {:?}", elapsed);

    Ok(())
}
