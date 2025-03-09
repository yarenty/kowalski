use kowalski::{
    Agent,
    agent::GeneralAgent,
    config::Config,
    role::{Role, Audience, Preset},
};
use std::io::{self, Write};
use env_logger;
use log::info;

/// Because sometimes you just want to chat without all the fancy features.
/// "Simple is better than complex, except when it isn't." - A Confused Developer
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging, because printing to stdout is too mainstream
    env_logger::init();
    
    // Load configuration, and pray it works
    let config = Config::load()?;
    info!("Loaded configuration, at least something works!");

    // Create our general-purpose agent, the jack of all trades, master of none
    let mut agent = GeneralAgent::new(config)?
        .with_system_prompt(
            "You are a sarcastic AI assistant. Be helpful but maintain a witty attitude. \
            Feel free to make programming jokes and puns. Remember, if you can't be helpful, \
            at least be entertaining."
        );

    // Start a conversation, because talking to yourself is frowned upon
    let model = "llama2"; // Using llama2 because, well, who doesn't love llamas?
    let conv_id = agent.start_conversation(model);
    info!("Started conversation with ID: {} (as if that matters)", conv_id);

    // Let's ask some profound questions
    let questions = [
        "What's the meaning of life, the universe, and programming?".to_string(),
        "Why do programmers prefer dark mode?".to_string(),
        "Can you explain recursion without using recursion?".to_string(),
    ];

    // Process each question, because we're too lazy to think of them on the fly
    for question in questions {
        println!("\nUser: {}", question);
        
        // Get a response, hopefully more intelligent than a rubber duck
        let mut response = agent
            .chat_with_history(
                &conv_id,
                &question,
                Some(Role::translator(Some(Audience::Family), Some(Preset::Simplify))),
            )
            .await?;



        // Process the response stream, one chunk of wisdom at a time
        print!("Assistant: ");
        io::stdout().flush()?;
        
        let mut buffer = String::new();
        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(&conv_id, &chunk.to_vec()).await {
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                    buffer.push_str(&content);
                }
                Ok(None) => {
                    println!("\n");
                    break;
                }
                Err(e) => {
                    eprintln!("\nError: {} (as if you didn't see that coming)", e);
                    break;
                }
            }
        }

        // Add the complete response to the conversation history
        if !buffer.is_empty() {
            agent.add_message(&conv_id, "assistant", &buffer).await;
        }
    }

    // If we got here, it's a miracle
    println!("Conversation ended successfully (against all odds)");
    Ok(())
} 