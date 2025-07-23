# Kowalski Core

The core library for the Kowalski AI agent framework, providing foundational abstractions, types, and utilities for building modular, extensible, and robust AI agents.

---

## Description

`kowalski-core` is the heart of the Kowalski agent ecosystem. It defines the essential building blocks for agent-based AI systems, including agent logic, conversation management, tool and toolchain orchestration, model management, roles/personas, configuration, error handling, and logging. All other Kowalski modules and agents build on top of these abstractions.

---

## Dependencies

- **serde** (with `derive`) — Serialization/deserialization
- **serde_json** — JSON support
- **async-trait** — Async trait support
- **tokio** — Async runtime
- **reqwest** — HTTP client
- **thiserror** — Error handling
- **uuid** — Unique IDs for conversations, etc.
- **chrono** — Date/time utilities
- **log** — Logging facade
- **config** — Configuration file parsing
- **dirs** — Platform-specific directory helpers
- **toml** — TOML parsing
- **env_logger** — Logging backend
- **url** — URL parsing

---

## Architecture

```
kowalski-core/
├── agent/         # Agent trait and base agent implementation
├── config.rs      # Configuration system
├── conversation/  # Conversation and message types
├── error.rs       # Unified error types
├── logging/       # Logging utilities
├── memory/        # Multi-tiered memory system
├── model/         # Model management and selection
├── role/          # Role, audience, preset, and style abstractions
├── tool_chain.rs  # Tool chain orchestration
├── tools.rs       # Tool trait and parameter types
├── utils/         # Utility helpers
└── lib.rs         # Main library entry point
```

- **Trait-based design**: All extensible components (agents, tools, task types) are defined as traits.
- **Async-first**: All major operations are async for scalability.
- **Strong typing**: Rich, serializable types for all core concepts.
- **Extensible**: Designed for easy extension with new tools, roles, and agent types.

---

## Core Functionality & Examples

### 1. Agent Abstraction

Defines the `Agent` trait and a `BaseAgent` implementation for managing conversations, interacting with models, and handling messages.

```rust
use kowalski_core::{Agent, BaseAgent, Config};

let config = Config::default();
let mut agent = BaseAgent::new(config, "Demo Agent", "A test agent").await?;
let conv_id = agent.start_conversation("llama3.2");
agent.add_message(&conv_id, "user", "Hello, world!").await;
```

---

### 2. Memory System

`kowalski-core` includes a sophisticated, multi-tiered memory system that gives agents a robust and scalable memory, moving beyond simple conversation history to enable true learning and context retention.

For a detailed explanation of the memory architecture, please see [MEMORY_ARCHITECTURE.md](./MEMORY_ARCHITECTURE.md).

---

### 3. Conversation Management

Manages conversation history, messages, and tool calls.

```rust
use kowalski_core::conversation::Conversation;

let mut conv = Conversation::new("llama3.2");
conv.add_message("user", "What's the weather?");
for msg in conv.get_messages() {
    println!("{}: {}", msg.role, msg.content);
}
```

---

### 4. Tool & Tool Chain System

Defines the `Tool` trait for pluggable tools and the `ToolChain` for orchestrating tool execution.

```rust
use kowalski_core::{Tool, ToolInput, ToolOutput, ToolChain};
use serde_json::json;

struct EchoTool;
#[async_trait::async_trait]
impl Tool for EchoTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, kowalski_core::KowalskiError> {
        Ok(ToolOutput::new(json!({"echo": input.content}), None))
    }
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes input" }
    fn parameters(&self) -> Vec<kowalski_core::ToolParameter> { vec![] }
}

let mut chain = ToolChain::new();
chain.register_tool(Box::new(EchoTool));
```

---

### 5. Model Management

Handles model listing, existence checks, and pulling models from a server.

```rust
use kowalski_core::model::ModelManager;

let manager = ModelManager::new("http://localhost:11434".to_string())?;
let models = manager.list_models().await?;
```

---

### 6. Roles, Audiences, Presets, Styles

Allows agents to assume different personas and communication styles.

```rust
use kowalski_core::role::{Role, Audience, Preset, Style};

let role = Role::new("Teacher", "Explains concepts simply")
    .with_audience(Audience::new("Student", "Learning Rust"))
    .with_preset(Preset::new("Beginner", "No prior experience"))
    .with_style(Style::new("Friendly", "Conversational and encouraging"));
```

---

### 7. Configuration

Flexible, extensible configuration system for agents and tools.

```rust
use kowalski_core::Config;

let config = Config::default();
println!("Ollama host: {}", config.ollama.host);
```

---

### 8. Error Handling

Unified error type for all core operations.

```rust
use kowalski_core::KowalskiError;

fn do_something() -> Result<(), KowalskiError> {
    Err(KowalskiError::ToolExecution("Something went wrong".into()))
}
```

---

## Future Enhancements

- **Agent orchestration**: Multi-agent collaboration and federation
- **Advanced tool chaining**: Conditional and parallel tool execution
- **Persistent conversation storage**: Database-backed conversation history
- **Dynamic model selection**: Automatic model switching based on context
- **Role learning**: Adaptive personas based on user feedback
- **Plugin system**: Hot-swappable tools and agent extensions
- **Improved logging and tracing**: Distributed tracing and analytics

---