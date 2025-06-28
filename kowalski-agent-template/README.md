# Kowalski Agent Template

A flexible foundation for building custom AI agents in the Kowalski ecosystem. This crate provides a robust agent base, builder patterns, configuration, and ready-to-use templates for rapid agent development.

---

## Description

`kowalski-agent-template` is designed to make it easy to create new, specialized AI agents by composing tools, task handlers, and configuration. It provides a `TemplateAgent` abstraction, a builder for ergonomic construction, and a set of templates (such as general-purpose and research agents) to jumpstart development.

---

## Dependencies

- **kowalski-core** — Core agent abstractions, tools, and types
- **kowalski-tools** — Ready-to-use tools (web search, PDF, data, code, etc.)
- **tokio** — Async runtime
- **reqwest** — HTTP client
- **serde** — Serialization/deserialization
- **serde_json** — JSON support
- **async-trait** — Async trait support
- **log** — Logging
- **thiserror** — Error handling
- **futures** — Async utilities
- **bytes** — Byte buffers
- **tower**, **tower-http** — Middleware and HTTP utilities
- **tracing**, **tracing-subscriber** — Structured logging and tracing
- **url** — URL parsing
- **regex** — Regular expressions
- **markdown** — Markdown parsing
- **env_logger** — Logging backend

---

## Architecture

```
kowalski-agent-template/
├── src/
│   ├── agent.rs        # TemplateAgent and TaskHandler traits
│   ├── builder.rs      # AgentBuilder for ergonomic agent construction
│   ├── config.rs       # TemplateAgentConfig and defaults
│   ├── lib.rs          # Library entry point
│   └── templates/      # Predefined agent templates
│       ├── general.rs  # General-purpose agent template
│       ├── research.rs # Research-focused agent template
│       └── mod.rs
```

- **TemplateAgent**: Core agent abstraction with tool/task handler registration and execution
- **AgentBuilder**: Builder pattern for ergonomic agent construction
- **Templates**: Predefined agent blueprints for common use cases
- **Config**: Extensible configuration for agent behavior and system prompts

---

## Core Functionality & Examples

### 1. TemplateAgent: The Extensible Agent

The `TemplateAgent` struct provides a flexible base for building custom agents. It supports:
- Tool registration (for pluggable capabilities)
- Task handler registration (for custom logic)
- System prompt configuration
- Async task execution

```rust
use kowalski_agent_template::agent::TemplateAgent;
use kowalski_core::tools::{Tool, ToolInput, ToolOutput};
use kowalski_core::config::Config;

let config = Config::default();
let agent = TemplateAgent::new(config).await?;
// Register tools and handlers as needed
```

---

### 2. AgentBuilder: Ergonomic Construction

The `AgentBuilder` pattern allows you to fluently compose an agent with custom tools, prompts, and settings.

```rust
use kowalski_agent_template::builder::AgentBuilder;
use kowalski_tools::web::WebSearchTool;

let builder = AgentBuilder::new()
    .await
    .with_system_prompt("You are a helpful assistant.")
    .with_tool(WebSearchTool::new("duckduckgo".to_string()))
    .with_temperature(0.5);

let agent = builder.build().await?;
```

---

### 3. Configuration

The `TemplateAgentConfig` struct provides flexible configuration for agent behavior, including concurrency, timeouts, user agent, and system prompt.

```rust
use kowalski_agent_template::config::TemplateAgentConfig;

let config = TemplateAgentConfig::default();
println!("System prompt: {}", config.system_prompt);
```

---

### 4. Templates: Ready-to-Use Agent Blueprints

#### GeneralTemplate
A general-purpose agent with basic tools (web search, PDF processing) and customizable prompt/temperature.

```rust
use kowalski_agent_template::templates::general::GeneralTemplate;

let builder = GeneralTemplate::create_default_agent().await?;
let agent = builder.build().await?;
```

You can also create a custom general agent:

```rust
let builder = GeneralTemplate::create_agent(
    vec![Box::new(WebSearchTool::new("duckduckgo".to_string()))],
    Some("You are a specialized assistant for web research.".to_string()),
    Some(0.5)
).await?;
let agent = builder.build().await?;
```

#### ResearchTemplate
A research-focused agent with web search and PDF analysis tools, and a research-oriented system prompt.

```rust
use kowalski_agent_template::templates::research::ResearchTemplate;

let builder = ResearchTemplate::create_agent().await?;
let agent = builder.build().await?;
```

---

## How to Extend

- **Add new tools**: Implement the `Tool` trait and register with your agent or builder.
- **Add new task handlers**: Implement the `TaskHandler` trait for custom logic.
- **Create new templates**: Compose new agent blueprints in the `templates/` directory.
- **Customize configuration**: Use or extend `TemplateAgentConfig` for new settings.

---

## Future Enhancements

- More agent templates for specific domains (e.g., coding, data analysis, customer support)
- Dynamic tool loading and plugin support
- Advanced orchestration (multi-agent, federated agents)
- Persistent agent state and conversation history
- Integration with external APIs and databases
- Enhanced configuration and environment support

--- 