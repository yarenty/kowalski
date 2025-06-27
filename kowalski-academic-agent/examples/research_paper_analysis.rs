use env_logger;
use kowalski_academic_agent::{agent::AcademicAgent, config::Config};
use kowalski_core::{
    agent::Agent,
    role::{Audience, Preset, Role},
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::load()?;
    let mut academic_agent = AcademicAgent::new(config)?;

    // Process a research paper
    println!("📚 Processing research paper...");
    let conversation_id = academic_agent.start_conversation(&config.ollama.model);
    println!("Academic Agent Conversation ID: {}", conversation_id);

    // Set up the role for scientific paper analysis
    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

    // Example paper path - replace with your PDF path
    let paper_path = "path/to/your/paper.pdf";
    println!("Processing paper: {}", paper_path);

    let mut response = academic_agent
        .chat_with_history(&conversation_id, paper_path, Some(role))
        .await?;

    println!("\n📝 Paper Analysis:");

    // Process the streaming response
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await? {
        match academic_agent
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
                academic_agent
                    .add_message(&conversation_id, "assistant", &buffer)
                    .await;
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
        match academic_agent
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
                academic_agent
                    .add_message(&conversation_id, "assistant", &buffer)
                    .await;
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
