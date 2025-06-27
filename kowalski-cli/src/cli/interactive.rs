use futures::StreamExt;
use kowalski_core::role::{Audience, Preset, Role};
use kowalski_core::Agent;
use kowalski_core::{config::Config, conversation::Message, model::ModelManager};
use std::io::{self, Write};
use std::path::PathBuf;


// Add missing type definitions
pub type ToolingAgent = Box<dyn Agent + Send + Sync>;
pub type GeneralAgent = Box<dyn Agent + Send + Sync>;


/// Stream generator that yields messages from the agent's response
async fn message_stream_generator<'a, A: Agent + 'a>(
    agent: &'a mut A,
    conv_id: &'a str,
    response: &'a mut reqwest::Response,
) -> impl futures::Stream<
    Item = Result<Option<kowalski_core::Message>, kowalski_core::error::KowalskiError>,
> + 'a {
    futures::stream::unfold((), move |_| {
        async move {
            let agent = unsafe { &mut *agent };
            let response = unsafe { &mut *response };
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    let result = agent.process_stream_response(conv_id, &chunk).await;
                    Some((result, ()))
                }
                Ok(None) => None,
                Err(e) => Some((
                    Err(kowalski_core::error::KowalskiError::Server(e.to_string())),
                    (),
                )),
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
            if let Some(args) = tool_call.function.arguments.as_object() {
                for (key, value) in args {
                    print!("{}: {}, ", key, value);
                }
            }
            println!(")");
            io::stdout().flush()?;
        }
    }

    Ok(())
}

/// Generic interaction loop for all agent types
pub async fn run_interaction_loop<A: Agent>(
    mut agent: A,
    mut conv_id: String,
    model: &str,
    initial_message: Option<&str>,
    role: Option<Role>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Session started with model: {} (type '/bye' to exit)", model);
    println!("----------------------------------------");

    if let Some(msg) = initial_message {
        println!("User: {}", msg);
        let mut response = agent.chat_with_history(&conv_id, msg, role.clone()).await?;
        let mut buffer = String::new();
        print!("Kowalski: ");
        let mut stream = Box::pin(message_stream_generator(&mut agent, &conv_id, &mut response).await);
        while let Some(result) = stream.next().await {
            match result {
                Ok(Some(message)) => {
                    process_message(message, &mut agent, &conv_id, &mut buffer).await?;
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

    loop {
        print!("User: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "/bye" {
            println!("Goodbye!");
            break;
        }

        if input.is_empty() {
            continue;
        }

        let mut response = agent.chat_with_history(&conv_id, input, role.clone()).await?;
        let mut buffer = String::new();
        print!("Kowalski: ");
        let mut stream = Box::pin(message_stream_generator(&mut agent, &conv_id, &mut response).await);
        while let Some(result) = stream.next().await {
            match result {
                Ok(Some(message)) => {
                    process_message(message, &mut agent, &conv_id, &mut buffer).await?;
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
        let mut agent = kowalski_core::BaseAgent::new(self.config.clone(), "Chat Agent", "A general purpose chat agent")?;
        let conv_id = agent.start_conversation(model);
        run_interaction_loop(agent, conv_id, model, None, None).await
    }

    /// List available models
    pub async fn list_models(&self) -> Result<(), Box<dyn std::error::Error>> {
        let models = self.model_manager.list_models().await?;
        for model in models {
            println!("{:?}", model);
        }
        Ok(())
    }

    /// Pull a model
    pub async fn pull_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self.model_manager.pull_model(name).await?;
        println!("Pull status for {}: {}", name, response.status);
        Ok(())
    }

    /// Delete a model
    pub async fn delete_model(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // self.model_manager.delete_model(name).await?;
        println!("Model {} deletion is not yet supported.", name);
        Ok(())
    }
}