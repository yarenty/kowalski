/// Main: The entry point of our AI-powered circus.
/// "Main functions are like orchestras - they make everything work together, but nobody notices until something goes wrong."
///
/// This is where the magic happens, or at least where we pretend it does.
/// Think of it as the conductor of our AI symphony, but with more error handling.
mod agent;
mod config;
mod conversation;
mod model;
mod role;
mod utils;
mod tools;
mod cli;

use clap::Parser;
use cli::Cli;
use env_logger;

/// The main function that makes everything work (or at least tries to).
/// "Main functions are like first dates - they're exciting but usually end in disappointment."
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute the command
    cli::execute(cli).await?;

    Ok(())
}
