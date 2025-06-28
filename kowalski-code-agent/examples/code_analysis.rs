use env_logger;
use kowalski_code_agent::{agent::CodeAgent, config::Config};
use kowalski_core::{
    agent::Agent,
    role::{Audience, Preset, Role},
};
use log::info;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::load()?;
    let mut code_agent = CodeAgent::new(config)?;

    // Start a conversation
    info!("🤖 Starting code agent...");
    let conversation_id = code_agent.start_conversation(&config.ollama.model);
    info!("Code Agent Conversation ID: {}", conversation_id);

    // Example code to analyze
    let code = r#"
    fn calculate_factorial(n: u32) -> u32 {
        if n <= 1 {
            return 1;
        }
        n * calculate_factorial(n - 1)
    }

    fn main() {
        let result = calculate_factorial(5);
        println!("Factorial of 5 is: {}", result);
    }
    "#;

    println!("\n🔍 Analyzing code...");

    // Analyze the code
    let analysis_result = code_agent.analyze_code(code).await?;
    println!("\n📊 Analysis Results:");
    println!("{}", analysis_result);

    // Refactor the code
    println!("\n🔄 Refactoring code...");
    let refactored_code = code_agent.refactor_code(code).await?;
    println!("\n📝 Refactored Code:");
    println!("{}", refactored_code);

    // Generate documentation
    println!("\n📚 Generating documentation...");
    let documentation = code_agent.document_code(code).await?;
    println!("\n📖 Generated Documentation:");
    println!("{}", documentation);

    // Search for similar code patterns
    println!("\n🔎 Searching for similar code patterns...");
    let search_results = code_agent.search_code("factorial function").await?;
    println!("\n🔍 Search Results:");
    println!("{}", search_results);

    // Add analysis results to conversation
    code_agent
        .add_message(
            &conversation_id,
            "analysis",
            format!("Code analysis results: {}", analysis_result).as_str(),
        )
        .await;

    // Generate a summary with a specific role
    let role = Role::translator(Some(Audience::Developer), Some(Preset::Technical));
    println!("\n📝 Generating technical summary...");

    let mut response = code_agent
        .chat_with_history(
            &conversation_id,
            "Provide a technical summary of the analysis",
            Some(role),
        )
        .await?;

    // Process the streaming response
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await? {
        match code_agent
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
                code_agent
                    .add_message(&conversation_id, "assistant", &buffer)
                    .await;
                println!("\n✅ Analysis complete!\n");
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
