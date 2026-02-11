# kowalski-core: Core Agent Framework

## 1. Purpose

The `kowalski-core` crate serves as the foundational library for the entire Kowalski multi-agent framework. It provides the essential abstractions, traits, and common implementations for building AI agents, managing conversations, integrating tools, and handling a multi-tiered memory system. It aims to be the robust, extensible, and high-performance backbone upon which all specialized Kowalski agents are built.

## 2. Structure

The `kowalski-core/src` directory is well-organized into logical modules:

*   **`agent/`**: Defines the core `Agent` trait, which all specialized agents must implement, and the `BaseAgent` struct, providing default implementations for common agent functionalities (conversation management, memory integration, tool execution loop).
*   **`conversation/`**: Manages the structure and history of conversations, including messages and models used.
*   **`config.rs`**: Handles configuration loading and management for the core framework.
*   **`error.rs`**: Defines custom error types (`KowalskiError`) for consistent error handling across the framework.
*   **`logging/`**: Provides logging and tracing utilities.
*   **`memory/`**: Implements the sophisticated multi-tiered memory architecture (Working, Episodic, Semantic), including a `MemoryProvider` trait and specific implementations for each tier. It also includes `consolidation.rs` for memory "weaving."
*   **`model/`**: Contains the `ModelManager` for interacting with LLM providers (currently tightly coupled to Ollama).
*   **`role/`**: Defines concepts like `Role`, `Audience`, `Preset`, and `Style` to guide LLM behavior.
*   **`template/`**: Provides mechanisms for creating agents from templates, exemplified by `DefaultTemplate`.
*   **`tools.rs`**: Defines the `Tool` trait, `ToolInput`, `ToolOutput`, `ToolCall`, and `ToolParameter` structs, which are fundamental for tool integration.
*   **`tool_chain.rs`**: Intends to manage a chain of tools, but its current implementation is a basic tool registry.

## 3. Strengths

*   **Robust Core Abstractions:** The `Agent` and `Tool` traits are well-defined and provide a strong foundation for building extensible agent functionalities.
*   **Sophisticated Memory Architecture:** The multi-tiered memory system (Working, Episodic, Semantic) is a significant strength, faithfully implemented with appropriate technologies (in-memory, RocksDB, Qdrant/petgraph). The hybrid retrieval and memory consolidation ("Memory Weaver") are advanced features.
*   **Modular Design:** The logical separation into distinct modules (`agent`, `memory`, `model`, `tools`, etc.) enhances maintainability and understanding.
*   **ReAct-style Tool Execution:** The `BaseAgent` includes a `chat_with_tools` method that implements a ReAct-style loop for LLM-driven tool use, which is a powerful pattern.
*   **Rust-native Performance and Safety:** Leveraging Rust ensures high performance, memory safety, and concurrency, crucial for an agentic framework.

## 4. Weaknesses

*   **Singleton Memory Providers:** The use of `tokio::sync::OnceCell` for `EpisodicBuffer` and `SemanticStore` creates singletons. This design choice severely limits the ability to:
    *   Run multiple independent agents within the same process without shared memory conflicts.
    *   Conduct isolated unit testing for memory components.
    *   Manage the lifecycle and configuration of memory providers dynamically.
    *   This is a critical architectural flaw for a multi-agent framework.
*   **Tool Management Inconsistencies:**
    *   The `tool_chain.rs` crate defines a `ToolChain` that is functionally a simple tool registry, not a chain, leading to misleading naming and limited capabilities.
    *   The `ToolManager` in `kowalski-tools` (which is a better design) is not integrated or used by `BaseAgent`, indicating a lack of centralized tool management.
    *   Hardcoded tool-chaining logic exists directly within `agent/mod.rs` (`chat_with_tools`), which is inflexible and duplicates concerns.
*   **LLM Provider Coupling:** The `ModelManager` is tightly coupled to Ollama, lacking a generic `ModelProvider` trait or similar abstraction to easily integrate other LLM services (e.g., OpenAI, Anthropic).
*   **JSON Parsing Fragility:** The logic for extracting JSON tool calls from raw LLM responses is somewhat brittle (simple `{` and `}` matching), which could lead to parsing failures with complex or malformed LLM outputs.
*   **Redundant Type Definitions:** Duplication of `ToolParameter` and `ParameterType` definitions exists between `kowalski-core` and `kowalski-tools`, which should be resolved to avoid confusion and maintain a single source of truth.

## 5. Potential Improvements & Integration into Rebuild

To align with a "total rebuild" aiming for "100x better than OpenClaw" while maintaining Kowalski's strengths, `kowalski-core` needs significant architectural refinement:

*   **Dependency Injection for Memory:** Refactor memory providers to use dependency injection (e.g., passing `Arc<dyn MemoryProvider>` or `Box<dyn MemoryProvider>`) to eliminate singletons, enable independent agent instances, and facilitate testing. This is the highest priority.
*   **Unified and Flexible Tool Management:**
    *   Develop a single, robust `ToolManager` (similar to the one in `kowalski-tools`) within `kowalski-core` that handles tool registration, discovery, validation, and execution.
    *   Introduce a flexible tool-chaining or "tool-orchestration" mechanism (e.g., using a directed acyclic graph or rule-based system) that is configurable and not hardcoded.
    *   Implement dynamic tool definition generation (e.g., OpenAPI-style JSON schema) that can be passed to LLMs, removing the need for manual prompt updates.
*   **LLM Provider Abstraction:** Introduce a `LLMProvider` trait (or similar) to abstract different LLM APIs, allowing for easy integration of OpenAI, Anthropic, Gemini, etc., alongside Ollama.
*   **Robust JSON Parsing:** Employ a more sophisticated JSON parsing library or strategy to reliably extract tool calls from LLM responses, even when embedded in natural language.
*   **Refactor `conversation` and `tool_chain`**: Review the commented out `conversation` module in `lib.rs` and the `ToolCall` ambiguity. Re-evaluate `tool_chain.rs`'s purpose; it might be better absorbed into a robust `ToolManager`.
*   **Consistent Error Handling:** Standardize the error handling strategy across all components.

These changes will make `kowalski-core` more robust, flexible, and scalable, laying a much stronger foundation for the advanced multi-agent and federation capabilities envisioned in the rebuild plan.