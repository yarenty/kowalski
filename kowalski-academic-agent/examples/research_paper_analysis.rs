use env_logger;
use kowalski_academic_agent::agent::AcademicAgent;
use kowalski_core::{
    agent::Agent,
    config::Config,
    role::{Audience, Preset, Role},
};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::default();
    let mut academic_agent = AcademicAgent::new(config).await?;

    // Process a research paper
    println!("📚 Processing research paper...");
    let conversation_id = academic_agent.start_conversation("llama3.2");
    println!("Academic Agent Conversation ID: {}", conversation_id);

    // Set up the role for scientific paper analysis
    let role = Role::new(
        "Academic Research Assistant",
        "You are an expert at analyzing academic papers and research.",
    )
    .with_audience(Audience::new(
        "Scientist",
        "You are speaking to a scientist who needs detailed analysis.",
    ))
    .with_preset(Preset::new(
        "Questions",
        "Ask clarifying questions to better understand the research.",
    ));

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
                        if let Some(obj) = tool_call.function.arguments.as_object() {
                            for (key, value) in obj {
                                print!("{}: {}, ", key, value);
                            }
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
                        if let Some(obj) = tool_call.function.arguments.as_object() {
                            for (key, value) in obj {
                                print!("{}: {}, ", key, value);
                            }
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
