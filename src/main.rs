mod agent;

use agent::{Agent, Message};
use std::io::{self, Write};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = Agent::new(None)?;

    let messages = vec![Message {
        role: "user".to_string(),
        content: "why is the sky blue?".to_string(),
    }];

    
    // Regular chat

    // let response = agent.chat("llama2", messages.clone()).await?;
    // println!("Regular chat response: {}", response.response);



    // Streaming chat
    let mut stream = agent.stream_chat("llama2", messages).await?;
    let mut buffer = String::new();
    
    println!("Streaming chat started...");

    while let Some(chunk) = stream.chunk().await? {
        // println!("Chunk: {:?}", chunk);
        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
            // print!("data: {}", text);
            let v: Value = serde_json::from_str(&text).unwrap();
            print!("{}", v["message"]["content"].as_str().unwrap());
            io::stdout().flush()?;
            buffer.push_str(&text);
        }
    }
    println!("\nStreaming chat complete!");

    Ok(())
}