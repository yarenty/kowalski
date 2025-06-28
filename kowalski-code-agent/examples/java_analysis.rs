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
    println!("☕ Starting Java Code Analysis...");
    let conversation_id = code_agent.start_conversation("llama3.2");
    println!("Code Agent Conversation ID: {}", conversation_id);

    // Set up the role for code analysis
    let role = Role::new(
        "Java Code Analysis Assistant",
        "You are an expert at analyzing Java code, providing insights on code quality, best practices, and potential improvements.",
    )
    .with_audience(Audience::new(
        "Java Developer",
        "You are speaking to a Java developer who needs detailed code analysis.",
    ))
    .with_preset(Preset::new(
        "Analysis",
        "Provide comprehensive analysis with specific recommendations for improvement.",
    ));

    // Sample Java code for analysis
    let java_code = r#"
import java.util.*;

public class Calculator {
    private int result;
    
    public Calculator() {
        this.result = 0;
    }
    
    public int add(int a, int b) {
        result = a + b;
        return result;
    }
    
    public int subtract(int a, int b) {
        result = a - b;
        return result;
    }
    
    public int multiply(int a, int b) {
        result = a * b;
        return result;
    }
    
    public double divide(int a, int b) {
        if (b == 0) {
            System.out.println("Error: Division by zero");
            return 0;
        }
        result = a / b;
        return (double) result;
    }
    
    public static void main(String[] args) {
        Calculator calc = new Calculator();
        System.out.println("Addition: " + calc.add(10, 5));
        System.out.println("Subtraction: " + calc.subtract(10, 5));
        System.out.println("Multiplication: " + calc.multiply(10, 5));
        System.out.println("Division: " + calc.divide(10, 5));
    }
}
"#;

    println!("\n📝 Java Code to Analyze:");
    println!("{}", java_code);

    // Analyze the Java code
    let analysis_result = code_agent.analyze_java(java_code).await?;

    println!("\n📊 Java Analysis Results:");
    println!("Language: {}", analysis_result.language);
    println!(
        "Metrics: {}",
        serde_json::to_string_pretty(&analysis_result.metrics)?
    );
    println!("Suggestions: {:?}", analysis_result.suggestions);
    println!("Issues: {:?}", analysis_result.issues);

    // Ask the agent to analyze the code
    let analysis_prompt = format!(
        "Please analyze this Java code and provide insights:\n\n{}\n\nAnalysis results:\nMetrics: {}\nSuggestions: {:?}\nIssues: {:?}",
        java_code,
        serde_json::to_string_pretty(&analysis_result.metrics)?,
        analysis_result.suggestions,
        analysis_result.issues
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

    // Ask a follow-up question about specific improvements
    let follow_up = "What specific improvements would you recommend for this Java code?";
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
