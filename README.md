# Kowalski

> "AI agents are like pets - they're cute but they make a mess." - Anonymous AI Developer
> "Programming is like writing a love letter to a computer that doesn't love you back." - Unknown

A Rust-based agent for interacting with Ollama models. Because apparently, we need another way to talk to AI.

## Project Overview

This project implements a basic agent that can communicate with Ollama's API, supporting both regular chat and streaming responses. It's built as a learning exercise and foundation for more complex agent implementations.

> "Simplicity is prerequisite for reliability." - Edsger W. Dijkstra

## Features

> "Features are like promises - they're great until you try to use them." - A Disappointed User

- 🤖 **Multiple Model Support**: Because one AI model is never enough
- 💬 **Conversation Management**: Keep track of your AI's ramblings
- 🎭 **Role-Based Interactions**: Give your AI a personality (or at least pretend to)
- 📝 **PDF and Text File Support**: Read files because typing is too mainstream
- 🔄 **Streaming Responses**: Watch your AI think in real-time (it's more exciting than it sounds)
- ⚙️ **Configurable Settings**: Customize everything until it breaks

## Installation

> "Installation is like cooking - it's easy until you burn something." - A Frustrated Developer

1. Clone the repository (because copying files manually is so last year):
   ```bash
   git clone https://github.com/yarenty/kowalski.git
   cd kowalski
   ```

2. Build the project (and pray it works):
   ```bash
   cargo build --release
   ```

3. Run the agent (and hope for the best):
   ```bash
   cargo run --release
   ```

## Usage

> "Usage instructions are like recipes - nobody reads them until something goes wrong." - A Support Agent

### Basic Usage

```rust
use kowalski::{agent::{AcademicAgent, ToolingAgent}, config::Config};

// Load configuration
let config = Config::load()?;

// Create agents (because one agent is never enough)
let academic_agent = AcademicAgent::new(config.clone())?;
let tooling_agent = ToolingAgent::new(config)?;

// Start conversations (double the fun, double the existential crisis)
let model_name = "llama2";
let academic_conv_id = academic_agent.start_conversation(model_name);
let tooling_conv_id = tooling_agent.start_conversation(model_name);
```

### General Chat

```rust
use kowalski::agent::GeneralAgent;

// Create a general-purpose chat agent
let general_agent = GeneralAgent::new(config.clone())?;

// Optionally customize the system prompt
let general_agent = general_agent.with_system_prompt(
    "You are a friendly and knowledgeable assistant. Help users with their questions."
);

// Start a conversation
let conv_id = general_agent.start_conversation("llama2");

// Simple chat interaction
let mut response = general_agent
    .chat_with_history(&conv_id, "What is the meaning of life?", None)
    .await?;

// Process streaming response
while let Some(chunk) = response.chunk().await? {
    match general_agent.process_stream_response(&conv_id, &chunk).await {
        Ok(Some(content)) => print!("{}", content),
        Ok(None) => break,
        Err(e) => eprintln!("Error: {}", e),
    }
}

// Continue the conversation with context
let mut response = general_agent
    .chat_with_history(&conv_id, "Can you elaborate on that?", None)
    .await?;
```

### Academic Research

```rust
use kowalski::role::{Role, Audience, Preset};

// Create a role for academic translation
let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

// Process a research paper
let mut response = academic_agent
    .chat_with_history(
        &academic_conv_id,
        "path/to/research.pdf",
        Some(role)
    )
    .await?;

// Process streaming response
while let Some(chunk) = response.chunk().await? {
    match academic_agent.process_stream_response(&academic_conv_id, &chunk).await {
        Ok(Some(content)) => print!("{}", content),
        Ok(None) => break,
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Web Research

```rust
// Perform web search
let query = "Latest developments in Rust programming";
let search_results = tooling_agent.search(query).await?;

// Process search results
for result in &search_results {
    println!("Title: {}", result.title);
    println!("URL: {}", result.url);
    println!("Snippet: {}", result.snippet);
}

// Fetch and analyze a webpage
if let Some(first_result) = search_results.first() {
    let page = tooling_agent.fetch_page(&first_result.url).await?;
    
    // Get a simplified summary
    let role = Role::translator(Some(Audience::Family), Some(Preset::Simplify));
    let mut response = tooling_agent
        .chat_with_history(&tooling_conv_id, "Provide simple summary", Some(role))
        .await?;
        
    // Process streaming response
    while let Some(chunk) = response.chunk().await? {
        match tooling_agent.process_stream_response(&tooling_conv_id, &chunk).await {
            Ok(Some(content)) => print!("{}", content),
            Ok(None) => break,
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
```

## Configuration

> "Configuration is like a relationship - it's complicated until you give up." - A System Administrator

The agent can be configured using a TOML file or environment variables:

```toml
[ollama]
base_url = "http://localhost:11434"
default_model = "mistral-small"

[chat]
temperature = 0.7
max_tokens = 512
stream = true
```

## Contributing

> "Contributing is like dating - it's fun until someone suggests changes." - An Open Source Maintainer

Contributions are welcome! Please feel free to submit a Pull Request. Just remember:
- Keep it clean (unlike my code)
- Add tests (because we all love writing tests)
- Update documentation (because reading code is so last year)

## License

> "Licenses are like prenuptial agreements - they're boring until you need them." - A Lawyer

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

> "Acknowledgments are like thank you notes - they're nice but nobody reads them." - A Grateful Developer

- Thanks to the Ollama team for making this possible
- Thanks to all contributors who helped make this project better
- Thanks to my coffee machine for keeping me awake during development 

## VISION

@see [features](FEATURES.md)

## ROADMAP

@see [roadmap](ROADMAP.md)


## Activity

![Alt](https://repobeats.axiom.co/api/embed/7ac42f1d632566d6dbc38b23cbdcd8c1881b3856.svg "Repobeats analytics image")