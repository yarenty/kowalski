use kowalski_academic_agent::agent::AcademicAgent;
use kowalski_core::role::{Audience, Preset, Role};
use kowalski_core::Agent;
use futures::StreamExt;
use std::io::{self, Write};
use std::path::PathBuf;
use kowalski_core::{
    config::Config,
    model::ModelManager,
    conversation::Message,
};
use log::info;

/// Stream generator that yields messages from the agent's response
async fn message_stream_generator<A: Agent>(
    agent: &mut A,
    conv_id: &str,
    response: &mut reqwest::Response,
) -> impl futures::Stream<Item = Result<Option<kowalski_core::Message>, kowalski_core::error::KowalskiError>> {
    futures::stream::unfold((), move |_| {
        let agent = agent as *mut A;
        let conv_id = conv_id.to_string();
        let response = response as *mut reqwest::Response;
        
        async move {
            unsafe {
                match (*response).chunk().await {
                    Ok(Some(chunk)) => {
                        let result = (*agent).process_stream_response(&conv_id, &chunk).await;
                        Some((result, ()))
                    }
                    Ok(None) => None,
                    Err(e) => Some((Err(kowalski_core::error::KowalskiError::Server(e.to_string())), ())),
                }
            }
        }
    })
}

/// Task processor that handles a single message
async fn process_message<A: Agent>(
    message: kowalski_core::Message,
    agent: &mut A,
    conv_id: &str,
    buffer: &mut String,
) -> Result<(), Box<dyn std::error::Error>> {
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

    Ok(())
}

/// Handles the continuous interaction loop with the AI
async fn interaction_loop<A: Agent>(
    mut agent: A,
    conv_id: &str,
    prompt: &str,
    role: Option<Role>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        print!("{}", prompt);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "/bye" {
            println!("Goodbye! (finally)");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Use the Ollama chat function
        ollama_chat(&mut agent, conv_id, input).await?;
    }

    Ok(())
}

/// Handles tool-based interaction with the AI
pub async fn tooling_loop(
    mut agent: ToolingAgent,
    conv_id: String,
    model: &str,
    initial_query: &str,
    tool_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{} started with model: {} (type '/bye' to exit)",
        tool_type, model
    );
    println!("----------------------------------------");

    // Initial query if provided
    if !initial_query.is_empty() {
        println!("Query: {}", initial_query);
        let mut response = agent
            .chat_with_history(&conv_id, initial_query, None)
            .await?;
        print!("Kowalski: ");
        let mut buffer = String::new();
        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(&conv_id, &chunk).await {
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
                            print!(")\n");
                            io::stdout().flush()?;
                        }
                    }
                }
                Ok(None) => {
                    println!("\n");
                    agent.add_message(&conv_id, "assistant", &buffer).await;
                    break;
                }
                Err(e) => {
                    eprintln!("\nError: {}", e);
                    break;
                }
            }
        }
    }

    // Use the common interaction loop with tool-specific prompt
    let prompt = match tool_type {
        "Search" => "Ask about your search query (type '/bye' to exit): ",
        "Scrape" => "What do you want to know more about your URL (type '/bye' to exit): ",
        "Code" => "Any a code-related question (type '/bye' to exit): ",
        _ => "Enter your query (or type '/bye' to exit): ",
    };

    interaction_loop(agent, &conv_id, prompt, None).await
}

/// Handles continuous chat interaction with the AI
pub async fn chat_loop(
    mut agent: GeneralAgent,
    conv_id: String,
    model: &str,
    initial_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Chat started with model: {} (type '/bye' to exit)", model);
    println!("----------------------------------------");

    // Initial message if provided
    if !initial_message.is_empty() {
        println!("User: {}", initial_message);
        let mut response = agent
            .chat_with_history(&conv_id, initial_message, None)
            .await?;

        let mut buffer = String::new();
        print!("Kowalski: ");
        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(&conv_id, &chunk).await {
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
                            print!(")\n");
                            io::stdout().flush()?;
                        }
                    }
                }
                Ok(None) => {
                    println!("\n");
                    agent.add_message(&conv_id, "assistant", &buffer).await;
                    break;
                }
                Err(e) => {
                    eprintln!("\nError: {}", e);
                    break;
                }
            }
        }
    }

    // Use the common interaction loop
    interaction_loop(agent, &conv_id, "User: ", None).await
}

/// Handles continuous academic paper analysis and Q&A
pub async fn academic_loop(
    mut agent: AcademicAgent,
    conv_id: String,
    model: &str,
    file: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Academic analysis started with model: {} (type '/bye' to exit)",
        model
    );
    println!("----------------------------------------");

    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

    let mut response = agent
        .chat_with_history(
            &conv_id,
            "provide me with a summary of the paper",
            Some(role.clone()),
        )
        .await?;

    print!("Kowalski: ");
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await? {
        match agent.process_stream_response(&conv_id, &chunk).await {
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
                println!("\n");
                agent.add_message(&conv_id, "assistant", &buffer).await;
                break;
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    // Use the common interaction loop with academic-specific prompt
    interaction_loop(agent, &conv_id, "Ask about the paper (type '/bye' to exit): ", Some(role)).await
}

/// Handles a single chat interaction with the AI
async fn ollama_chat<A: Agent>(
    mut agent: A,
    conv_id: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut response = agent.chat_with_history(conv_id, message, None).await?;

    let mut buffer = String::new();
    print!("Kowalski: ");
    while let Some(chunk) = response.chunk().await? {
        match agent.process_stream_response(conv_id, &chunk).await {
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
                println!("\n");
                agent.add_message(conv_id, "assistant", &buffer).await;
                break;
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Core interactive mode for basic LLM interactions
pub struct InteractiveMode {
    config: Config,
    model_manager: ModelManager,
}

impl InteractiveMode {
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let model_manager = ModelManager::new(config.ollama.host.clone())?;
        Ok(Self {
            config,
            model_manager,
        })
    }

    /// Start an interactive chat session
    pub async fn chat_loop(&self, model: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting chat session with model: {}", model);
        println!("Type '/bye' to exit, '/help' for commands");

        let mut buffer = String::new();
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            print!("> ");
            stdout.flush()?;
            buffer.clear();
            stdin.read_line(&mut buffer)?;
            let input = buffer.trim();

            match input {
                "/bye" => break,
                "/help" => {
                    println!("Available commands:");
                    println!("  /bye   - Exit the chat session");
                    println!("  /help  - Show this help message");
                    println!("  /model - Show current model info");
                }
                "/model" => {
                    println!("Current model: {}", model);
                }
                _ if !input.is_empty() => {
                    // Here we'll delegate to the specific agent implementation
                    // This will be handled by the agent-specific code
                    println!("Processing message: {}", input);
                }
                _ => continue,
            }
        }

        Ok(())
    }

    /// List available models
    pub async fn list_models(&self) -> Result<(), Box<dyn std::error::Error>> {
        let models = self.model_manager.list_models().await?;
        for model in models {
            println!("{}", model);
        }
        Ok(())
    }

    /// Pull a model
    pub async fn pull_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = self.model_manager.pull_model(name).await?;
        while let Some(response) = stream.next().await {
            match response {
                Ok(chunk) => print!("{}", chunk),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        Ok(())
    }

    /// Delete a model
    pub async fn delete_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.model_manager.delete_model(name).await?;
        println!("Model {} deleted", name);
        Ok(())
    }
} 