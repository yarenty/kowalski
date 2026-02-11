# kowalski-data-agent: Specialized Agent for Data Analysis

## 1. Purpose

The `kowalski-data-agent` crate provides a specialized AI agent focused on data analysis and processing tasks, particularly for CSV files. It integrates tools from `kowalski-tools` (specifically `CsvTool` and `FsTool`) to enable agents to read, summarize, and extract insights from tabular data. This agent serves as a strong example of building a domain-specific agent within the Kowalski framework, showcasing a more dynamic approach to tool management.

## 2. Structure

The `kowalski-data-agent/src` directory is concise:

*   **`agent.rs`**: Contains the `DataAgent` implementation. Similar to other specialized agents, it wraps a `TemplateAgent` (from `kowalski-core`) but notably includes logic to *dynamically generate* the tool definitions for its system prompt.
*   **`config.rs`**: Defines configuration settings specific to the data agent, such as `max_rows` and `max_columns` for CSV processing.
*   **`lib.rs`**: Re-exports the main components of the crate and common types from `kowalski-core`.

## 3. Strengths

*   **Dynamic Tool Prompt Generation:** This is the most significant strength of the `DataAgent`. It dynamically builds the "AVAILABLE TOOLS" section of its system prompt by introspecting the registered tools. This approach eliminates the brittleness of manually written prompts, ensures consistency, and reduces maintenance overhead when tool specifications change. This is a best practice that should be adopted across the entire framework.
*   **Clear Specialization:** The agent has a well-defined purpose in data analysis, primarily focusing on CSV data, making its capabilities clear.
*   **Leverages Core Framework:** Effectively utilizes the `Agent` trait and `TemplateAgent` from `kowalski-core`, demonstrating robust composition for specialized agent creation.
*   **Practical Tool Integration:** Integrates `CsvTool` and `FsTool` from `kowalski-tools`, providing essential functionalities for data handling.
*   **Specific Instructions in Prompt:** The system prompt is highly specific, providing clear instructions to the LLM on tool usage patterns and expected response formats, which can significantly improve tool-use reliability.

## 4. Weaknesses

*   **`TemplateAgent` Abstraction Leak:** The `DataAgent` continues to rely on `TemplateAgent`, an internal component of `kowalski-core` that is not publicly exposed. This inconsistency can hinder external developers' understanding and ability to replicate or customize agents effectively.
*   **Hardcoded Tool Creation (Still Present):** Although the prompt generation is dynamic, the actual instantiation of `CsvTool` and `FsTool` still occurs directly within the `DataAgent::new` function. This limits the flexibility for changing tool implementations or parameters without modifying the agent's source code directly.
*   **Limited Data Analysis Capabilities:** While the `CsvTool` provides basic summaries, more advanced data analysis (e.g., statistical modeling, visualization, integration with other data sources, handling different file formats) would require significant extensions.

## 5. Potential Improvements & Integration into Rebuild

To elevate `kowalski-data-agent` and establish it as a prime example for future Kowalski agents:

*   **Formalize Dynamic Tool Management:** The dynamic prompt generation logic should be extracted and formalized as part of the proposed unified `ToolManager` within `kowalski-core`. This would centralize this best practice and make it available to all agents by default.
*   **Dependency Injection for Tools:** Implement a mechanism to inject tools into the `DataAgent` (and other agents) rather than creating them directly. This could be achieved through a configuration system or a dedicated tool factory, greatly enhancing flexibility and testability.
*   **Refined Base Agent or Public Template:** As with other agents, address the `TemplateAgent` abstraction. Either make `TemplateAgent` public or refactor `DataAgent` to directly use `BaseAgent` with explicit tool management.
*   **Expand Data Formats:** Extend the `kowalski-tools` with support for other data formats (e.g., JSON, Excel, Parquet) and integrate them into the `DataAgent`.
*   **Advanced Data Analysis Tools:** Integrate with more sophisticated data science libraries or external tools for advanced statistical analysis, machine learning, and data visualization.
*   **Data Validation and Cleaning:** Develop tools that can automatically detect and suggest fixes for common data quality issues.

By adopting these improvements, the `kowalski-data-agent` can become a highly flexible, powerful, and exemplary component, embodying the "100x better" vision for the Kowalski framework.