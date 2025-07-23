use env_logger;
use kowalski_code_agent::agent::CodeAgent;
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
    let mut code_agent = CodeAgent::new(config).await?;

    // Start a conversation
    println!("🐍 Starting Python Code Analysis...");
    let conversation_id = code_agent.start_conversation("llama3.2");
    println!("Code Agent Conversation ID: {}", conversation_id);

    // Set up the role for code analysis
    let role = Role::new(
        "Python Code Analysis Assistant",
        "You are an expert at analyzing Python code, providing insights on code quality, PEP 8 compliance, and potential improvements.",
    )
    .with_audience(Audience::new(
        "Python Developer",
        "You are speaking to a Python developer who needs detailed code analysis.",
    ))
    .with_preset(Preset::new(
        "Analysis",
        "Provide comprehensive analysis with specific recommendations for improvement.",
    ));

    // Sample Python code for analysis
    let python_code = r#"
import os
import sys
from typing import List, Optional

class DataProcessor:
    def __init__(self, data: List[int]):
        self.data = data
        self.result = 0
    
    def calculate_sum(self) -> int:
        """Calculate the sum of all data points."""
        total = 0
        for item in self.data:
            total += item
        return total
    
    def calculate_average(self) -> float:
        """Calculate the average of all data points."""
        if len(self.data) == 0:
            print("Error: No data to calculate average")
            return 0.0
        return self.calculate_sum() / len(self.data)
    
    def find_max(self) -> Optional[int]:
        """Find the maximum value in the data."""
        if not self.data:
            return None
        max_val = self.data[0]
        for item in self.data:
            if item > max_val:
                max_val = item
        return max_val

def main():
    # Sample data
    numbers = [10, 20, 30, 40, 50]
    
    # Create processor
    processor = DataProcessor(numbers)
    
    # Calculate statistics
    print(f"Sum: {processor.calculate_sum()}")
    print(f"Average: {processor.calculate_average()}")
    print(f"Maximum: {processor.find_max()}")
    
    # Process empty data
    empty_processor = DataProcessor([])
    print(f"Empty average: {empty_processor.calculate_average()}")

if __name__ == "__main__":
    main()
"#;

    println!("\n📝 Python Code to Analyze:");
    println!("{}", python_code);

    // Analyze the Python code using tool-calling
    let analysis_input = format!(
        r#"{{"name": "python_analysis", "parameters": {{"content": {}}}}}"#,
        serde_json::to_string(python_code)?
    );
    let analysis_result = code_agent
        .chat_with_tools(&conversation_id, &analysis_input)
        .await?;
    println!("\n📊 Python Analysis Results:\n{}", analysis_result);

    // Ask the agent to analyze the code
    let analysis_prompt = format!(
        "Please analyze this Python code and provide insights:\n\n{}\n\nAnalysis results:\n{}\n",
        python_code, analysis_result
    );

    let mut response = code_agent
        .chat_with_history(&conversation_id, &analysis_prompt, Some(role))
        .await?;

    println!("\n🤖 AI Analysis:");

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

    // Ask a follow-up question about PEP 8 compliance
    let follow_up = "How can this Python code be improved to better follow PEP 8 guidelines?";
    let mut follow_up_response = code_agent
        .chat_with_history(&conversation_id, follow_up, None)
        .await?;

    println!("\n🔍 Follow-up Analysis:");
    let mut buffer = String::new();
    while let Some(chunk) = follow_up_response.chunk().await? {
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
                code_agent
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
