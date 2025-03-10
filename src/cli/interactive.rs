use crate::agent::{Agent, AcademicAgent, GeneralAgent, ToolingAgent};
use crate::role::{Audience, Preset, Role};
use crate::utils::PdfReader;
use std::io::{self, Write};
use std::path::PathBuf;

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

        let mut response = agent
            .chat_with_history(conv_id, input, role.clone())
            .await?;

        print!("Kowalski: ");
        io::stdout().flush()?;
        let mut buffer = String::new();

        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(conv_id, &chunk).await {
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                    buffer.push_str(&content);
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
    println!("{} started with model: {} (type '/bye' to exit)", tool_type, model);
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
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                    buffer.push_str(&content);
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
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                    buffer.push_str(&content);
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
    println!("Academic analysis started with model: {} (type '/bye' to exit)", model);
    println!("----------------------------------------");

    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

    // agent.add_role(&conv_id, Some(role));

    // agent.add_paper(&conv_id, file.as_path().to_str().unwrap());



    // dbg!(&content);

    let mut response = agent
        .chat_with_history(&conv_id, "provide me with a summary of the paper", Some(role.clone()))
        .await?;

        print!("Kowalski: ");

        let mut buffer = String::new();    
    io::stdout().flush()?;


    while let Some(chunk) = response.chunk().await? {
        match agent.process_stream_response(&conv_id, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
                buffer.push_str(&content);
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

    // Use the common interaction loop with the academic role
    interaction_loop(agent, &conv_id, "\nAsk a question about the paper (or type '/bye' to exit): ", Some(role)).await
} 