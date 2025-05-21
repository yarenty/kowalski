/// Main: The entry point of our AI-powered circus.
/// "Main functions are like orchestras - they make everything work together, but nobody notices until something goes wrong."
///
/// This is where the magic happens, or at least where we pretend it does.
/// Think of it as the conductor of our AI symphony, but with more error handling.
use clap::Parser;
use kowalski_cli::cli::{Cli, execute};
use kowalski_core::logging;

/// The main function that makes everything work (or at least tries to).
/// "Main functions are like first dates - they're exciting but usually end in disappointment."
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    kowalski_core::logging::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Execute the command
    execute(cli).await?;

    Ok(())
}
