# kowalski-academic-agent: Specialized Agent for Academic Research

## 1. Purpose

The `kowalski-academic-agent` crate provides a specialized AI agent designed for academic research tasks. It leverages tools from `kowalski-tools` (specifically web search, PDF processing, and filesystem operations) to assist users with activities like finding academic papers, analyzing PDF content, and managing research-related files. It serves as an example of how to build a domain-specific agent on top of the `kowalski-core` framework.

## 2. Structure

The `kowalski-academic-agent/src` directory is straightforward:

*   **`agent.rs`**: Contains the `AcademicAgent` implementation, which wraps a `TemplateAgent` (from `kowalski-core`) and configures it with academic-specific tools and a system prompt.
*   **`config.rs`**: Defines configuration settings specific to the academic agent.
*   **`error.rs`**: Defines custom error types for this agent.
*   **`lib.rs`**: Re-exports the main components of the crate and common types from `kowalski-core`.

## 3. Strengths

*   **Clear Specialization:** The agent has a well-defined purpose, focusing on academic research, making it easy to understand its scope and capabilities.
*   **Leverages Core Framework:** It demonstrates effective use of the `Agent` trait and `TemplateAgent` from `kowalski-core`, showcasing how to build specialized agents through composition.
*   **Tool Integration:** Successfully integrates `WebSearchTool`, `PdfTool`, and `FsTool` from `kowalski-tools` to provide relevant functionalities for its domain.
*   **Descriptive System Prompt:** The system prompt is well-crafted, clearly instructing the LLM on its role, available tools, and expected response format for academic tasks.

## 4. Weaknesses

*   **Hardcoded Tool Creation:** Tools are instantiated directly within the `AcademicAgent::new` function. This approach lacks flexibility, making it difficult to configure different sets of tools or vary tool parameters without modifying the agent's source code. A more externalized or configurable mechanism for tool instantiation would be beneficial.
*   **Manually Managed Tool List in Prompt:** The list of available tools and their usage instructions are hardcoded within the agent's system prompt. This makes the prompt brittle; any change to a tool's name, description, or parameters requires manual updates to the prompt, which is prone to errors and difficult to maintain.
*   **`TemplateAgent` Abstraction Leak:** The `AcademicAgent` relies on `TemplateAgent`, which is an internal component of `kowalski-core` and not publicly exposed. This "leak" makes it harder for external developers to understand or replicate the agent's internal workings without diving into `kowalski-core`'s internals. It also limits the flexibility of what a specialized agent can compose from.
*   **Limited Error Handling:** The error handling specifically within this agent appears basic, mostly relying on `KowalskiError` propagation. More granular, academic-specific error handling might be beneficial for a specialized agent.

## 5. Potential Improvements & Integration into Rebuild

To make `kowalski-academic-agent` more robust, flexible, and exemplary for the new Kowalski framework:

*   **Configurable Tool Instantiation:** Instead of hardcoding tool creation, implement a configuration-driven approach (e.g., using a `config.toml` or JSON file) that specifies which tools to instantiate and with what parameters. This would allow users to easily customize the agent's toolkit without recompilation.
*   **Dynamic Tool Prompt Generation:** Integrate with the proposed unified `ToolManager` (from `kowalski-core`) to dynamically generate the "AVAILABLE TOOLS" section of the system prompt. This would ensure consistency, reduce maintenance overhead, and automatically reflect any changes in tool definitions.
*   **Refined Base Agent or Public Template:**
    *   If `TemplateAgent` is a core concept, it should be made public and well-documented in `kowalski-core`.
    *   Alternatively, `AcademicAgent` (and other specialized agents) could directly compose `BaseAgent` and explicitly handle tool registration and prompt generation, providing more control and clarity.
*   **Enhanced Error Context:** Provide more specific error messages or custom error types for academic-specific failures, improving debuggability.
*   **Expand Toolset:** Once the `PdfTool` in `kowalski-tools` is fully implemented, the `AcademicAgent` can be enhanced to perform deeper analysis, citation extraction, and cross-referencing within academic documents.

These improvements would make the `kowalski-academic-agent` a more adaptable and powerful tool for academic research, showcasing the full potential of a refined Kowalski framework.