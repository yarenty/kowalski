use env_logger;
use kowalski_core::{
    agent::Agent,
    config::Config,
    role::{Audience, Preset, Role},
};
use kowalski_data_agent::agent::DataAgent;
use std::io::{self, Write};
use tempfile::NamedTempFile;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = Config::default();
    let mut data_agent = DataAgent::new(config).await?;

    // Start a conversation
    println!("📊 Starting CSV Analysis...");
    let conversation_id = data_agent.start_conversation("llama3.2");
    println!("Data Agent Conversation ID: {}", conversation_id);

    // Set up the role for data analysis
    let role = Role::new(
        "Data Analysis Assistant",
        "You are an expert at analyzing and interpreting data from CSV files.",
    )
    .with_audience(Audience::new(
        "Data Scientist",
        "You are speaking to a data scientist who needs detailed analysis.",
    ))
    .with_preset(Preset::new(
        "Analysis",
        "Provide comprehensive analysis with insights and recommendations.",
    ));

    // Sample CSV data for analysis
    let csv_data = r#"name,age,city,salary,department
John Doe,30,New York,75000,Engineering
Jane Smith,28,San Francisco,85000,Marketing
Bob Johnson,35,Chicago,65000,Sales
Alice Brown,32,Boston,70000,Engineering
Charlie Wilson,29,Seattle,80000,Engineering
Diana Davis,31,Austin,72000,Marketing
Eve Miller,27,Denver,68000,Sales
Frank Garcia,33,Portland,75000,Engineering
Grace Lee,26,Atlanta,65000,Marketing
Henry Taylor,34,Dallas,78000,Engineering"#;

    println!("\n📈 Processing CSV Data:");
    println!("{}", csv_data);

    // Write CSV data to a temporary file
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", csv_data)?;
    let temp_path = temp_file.path().to_str().unwrap();

    // Process the CSV file using the new method
    let analysis_summary = data_agent.process_csv_path(temp_path).await?;

    println!("\n📊 CSV Analysis Summary:");
    println!("{}", serde_json::to_string_pretty(&analysis_summary)?);

    // Always follow up with a prompt to the agent for natural language analysis
    let analysis_prompt = format!(
        "This is follow up. Given the following summary (in JSON), provide a detailed, human-readable analysis with key insights, trends, and recommendations. Do not ask for a file path. Here is the summary:\n\n{}",
        serde_json::to_string_pretty(&analysis_summary)?
    );

    println!("{}", &analysis_prompt);

    let mut response = data_agent
        .chat_with_history(&conversation_id, &analysis_prompt, Some(role))
        .await?;

    println!("\n🤖 AI Analysis:");

    // Process the streaming response
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await? {
        match data_agent
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
                data_agent
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

    // Ask a follow-up question about specific insights
    let follow_up = "What are the key insights about salary distribution across departments?";
    let mut follow_up_response = data_agent
        .chat_with_history(&conversation_id, follow_up, None)
        .await?;

    println!("\n🔍 Follow-up Analysis:");
    let mut buffer = String::new();
    while let Some(chunk) = follow_up_response.chunk().await? {
        match data_agent
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
                data_agent
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
