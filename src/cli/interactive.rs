use crate::agent::{Agent, AcademicAgent, GeneralAgent};
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

        print!("Assistant: ");
        io::stdout().flush()?;

        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(conv_id, &chunk).await {
                Ok(Some(content)) => {
                    print!("{}", content);
                    io::stdout().flush()?;
                }
                Ok(None) => {
                    println!("\n");
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

        while let Some(chunk) = response.chunk().await? {
            match agent.process_stream_response(&conv_id, &chunk).await {
                Ok(Some(content)) => {
                    print!("Assistant: {}", content);
                    io::stdout().flush()?;
                }
                Ok(None) => {
                    println!("\n");
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

    // Read the file content
    let content = if file.extension().map_or(false, |ext| ext == "pdf") {
        PdfReader::read_pdf_file(&file.to_string_lossy())?
    } else {
        std::fs::read_to_string(file)?
    };

    // Initial analysis of the file
    println!("Analyzing file: {}", file.display());
    let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));
    let mut response = agent
        .chat_with_history(&conv_id, &content, Some(role.clone()))
        .await?;

    print!("Assistant: ");
    io::stdout().flush()?;

    while let Some(chunk) = response.chunk().await? {
        match agent.process_stream_response(&conv_id, &chunk).await {
            Ok(Some(content)) => {
                print!("{}", content);
                io::stdout().flush()?;
            }
            Ok(None) => {
                println!("\n");
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