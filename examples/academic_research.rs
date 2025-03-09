use kowalski::{
    agent::{Agent, AcademicAgent},
    config::Config,
    role::{Audience, Preset, Role},
};
use std::io::{self, Write};
use env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::load()?;
    let mut academic_agent = AcademicAgent::new(config)?;

    // Process a research paper
    println!("📚 Processing research paper...");
    let conversation_id = academic_agent.start_conversation("llama2");
    println!("Academic Agent Conversation ID: {}", conversation_id);

    // Set up the role for scientific paper analysis
    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
    
    // Example paper path - replace with your PDF path
    let paper_path = "path/to/your/paper.pdf";
    println!("Processing paper: {}", paper_path);

    let mut response = academic_agent
        .chat_with_history(
            &conversation_id,
            paper_path,
            Some(role),
        )
        .await?;

    println!("\n📝 Paper Analysis:");

    // Process the streaming response
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await? {
        match academic_agent.process_stream_response(&conversation_id, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(&content);
            }
            Ok(None) => {
                academic_agent.add_message(&conversation_id, "assistant", &buffer).await;
                println!("\n✅ Paper processing complete!\n");
                break;
            }
            Err(e) => {
                eprintln!("\n❌ Error processing stream: {}", e);
                break;
            }
        }
    }

    // You can continue the conversation with follow-up questions
    let mut follow_up = academic_agent
        .chat_with_history(
            &conversation_id,
            "What are the main contributions of this paper?",
            None,
        )
        .await?;

    println!("\n📝 Follow-up Analysis:");
    let mut buffer = String::new();
    while let Some(chunk) = follow_up.chunk().await? {
        match academic_agent.process_stream_response(&conversation_id, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(&content);
            }
            Ok(None) => {
                academic_agent.add_message(&conversation_id, "assistant", &buffer).await;
                println!("\n");
                break;
            }
            Err(e) => {
                eprintln!("\n❌ Error processing stream: {}", e);
                break;
            }
        }
    }

    Ok(())
} 