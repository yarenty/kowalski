# Kowalski

A comprehensive Rust-based agent framework for interacting with Ollama models and building AI-powered applications.

## Overview

Kowalski is a modular framework that provides everything you need to build sophisticated AI agents. The framework is organized into specialized crates that can be used independently or together.

## Architecture

### Core Components
- **`kowalski-core`**: Basic agent infrastructure, types, and utilities
- **`kowalski-agent-template`**: Templates and builders for creating custom agents
- **`kowalski-tools`**: Collection of tools for web scraping, data processing, and more
- **`kowalski-federation`**: Multi-agent coordination and communication

### Specialized Agents
- **`kowalski-academic-agent`**: Research and academic paper analysis
- **`kowalski-code-agent`**: Code analysis, refactoring, and generation
- **`kowalski-data-agent`**: Data analysis and processing (optional feature)
- **`kowalski-web-agent`**: Web research and information gathering

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kowalski = "0.5.0"

# Optional: Enable data analysis capabilities
kowalski = { version = "0.5.0", features = ["data"] }
```

## Quick Start

```rust
use kowalski::{Agent, AgentBuilder, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a basic agent
    let agent = AgentBuilder::new()
        .with_model("llama2")
        .with_system_prompt("You are a helpful AI assistant.")
        .build()?;

    // Send a message
    let response = agent.send_message("Hello, how are you?").await?;
    println!("Response: {}", response.content);

    Ok(())
}
```

## Using Specialized Agents

### Academic Research
```rust
use kowalski::academic_agent::AcademicAgent;

let agent = AcademicAgent::new("llama2")?;
let analysis = agent.analyze_paper("path/to/paper.pdf").await?;
```

### Code Analysis
```rust
use kowalski::code_agent::CodeAgent;

let agent = CodeAgent::new("codellama")?;
let refactored = agent.refactor_code("src/main.rs").await?;
```

### Web Research
```rust
use kowalski::web_agent::WebAgent;

let agent = WebAgent::new("llama2")?;
let research = agent.research_topic("Rust async programming").await?;
```

## Features

- **Modular Design**: Use only the components you need
- **Async/Await**: Built on Tokio for high-performance async operations
- **Tool Integration**: Rich ecosystem of tools for various tasks
- **Multi-Agent Coordination**: Build complex systems with multiple agents
- **Extensible**: Easy to add custom tools and agents

## Documentation

- [API Documentation](https://docs.rs/kowalski)
- [Examples](https://github.com/yarenty/kowalski/tree/main/examples)
- [Contributing](https://github.com/yarenty/kowalski/blob/main/CONTRIBUTING.md)

## License

MIT License - see [LICENSE](LICENSE) file for details. 