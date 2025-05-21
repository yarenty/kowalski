use env_logger;
use kowalski_cli::cli::{Cli, execute};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the command
    execute(cli).await
}
