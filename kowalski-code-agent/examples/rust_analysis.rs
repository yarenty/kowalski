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
    println!("🦀 Starting Rust Code Analysis...");
    let conversation_id = code_agent.start_conversation("llama3.2");
    println!("Code Agent Conversation ID: {}", conversation_id);

    // Set up the role for code analysis
    let role = Role::new(
        "Rust Code Analysis Assistant",
        "You are an expert at analyzing Rust code, providing insights on code quality, safety, and potential improvements.",
    )
    .with_audience(Audience::new(
        "Rust Developer",
        "You are speaking to a Rust developer who needs detailed code analysis.",
    ))
    .with_preset(Preset::new(
        "Analysis",
        "Provide comprehensive analysis with specific recommendations for improvement.",
    ));

    // Sample Rust code for analysis
    let rust_code = r#"
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug)]
struct DataProcessor {
    data: Vec<i32>,
    cache: HashMap<String, i32>,
}

impl DataProcessor {
    fn new(data: Vec<i32>) -> Self {
        Self {
            data,
            cache: HashMap::new(),
        }
    }
    
    fn calculate_sum(&self) -> i32 {
        self.data.iter().sum()
    }
    
    fn calculate_average(&self) -> Option<f64> {
        if self.data.is_empty() {
            None
        } else {
            Some(self.calculate_sum() as f64 / self.data.len() as f64)
        }
    }
    
    fn find_max(&self) -> Option<&i32> {
        self.data.iter().max()
    }
    
    fn process_with_cache(&mut self, key: String) -> Result<i32, Box<dyn Error>> {
        if let Some(&cached_value) = self.cache.get(&key) {
            return Ok(cached_value);
        }
        
        let result = self.calculate_sum();
        self.cache.insert(key, result);
        Ok(result)
    }
}

fn main() {
    let numbers = vec![10, 20, 30, 40, 50];
    let mut processor = DataProcessor::new(numbers);
    
    println!("Sum: {}", processor.calculate_sum());
    
    match processor.calculate_average() {
        Some(avg) => println!("Average: {}", avg),
        None => println!("No data to calculate average"),
    }
    
    match processor.find_max() {
        Some(max) => println!("Maximum: {}", max),
        None => println!("No data to find maximum"),
    }
    
    match processor.process_with_cache("sum".to_string()) {
        Ok(result) => println!("Cached result: {}", result),
        Err(e) => println!("Error: {}", e),
    }
}
"#;

    println!("\n📝 Rust Code to Analyze:");
    println!("{}", rust_code);

    // Analyze the Rust code
    let analysis_result = code_agent.analyze_rust(rust_code).await?;

    println!("\n📊 Rust Analysis Results:");
    println!("Language: {}", analysis_result.language);
    println!(
        "Metrics: {}",
        serde_json::to_string_pretty(&analysis_result.metrics)?
    );
    println!("Suggestions: {:?}", analysis_result.suggestions);
    println!("Rust Issues: {:?}", analysis_result.issues);

    // Ask the agent to analyze the code
    let analysis_prompt = format!(
        "Please analyze this Rust code and provide insights:\n\n{}\n\nAnalysis results:\nMetrics: {}\nSuggestions: {:?}\nRust Issues: {:?}",
        rust_code,
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

    // Ask a follow-up question about Rust-specific improvements
    let follow_up = "What Rust-specific improvements would you recommend for this code?";
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
