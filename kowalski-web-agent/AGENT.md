# kowalski-web-agent: Specialized Agent for Web Research and Interaction

## 1. Purpose

The `kowalski-web-agent` crate provides a specialized AI agent focused on interacting with the web. Its primary function is to perform web searches and scrape content from specified URLs, enabling it to gather up-to-date information and process web-based data. It serves as an example of a domain-specific agent built upon the `kowalski-core` framework, utilizing tools from `kowalski-tools` for web functionalities.

## 2. Structure

The `kowalski-web-agent/src` directory is concise:

*   **`agent.rs`**: Contains the `WebAgent` implementation, which wraps a `TemplateAgent` (from `kowalski-core`) and configures it with web-specific tools and a system prompt.
*   **`config.rs`**: Defines configuration settings specific to the web agent, such including default search providers.
*   **`error.rs`**: Defines custom error types for this agent.
*   **`lib.rs`**: Re-exports the main components of the crate and common types from `kowalski-core`.

## 3. Strengths

*   **Clear Specialization:** The agent has a well-defined purpose in web research and interaction, making its capabilities clear and focused.
*   **Leverages Core Framework:** Effectively utilizes the `Agent` trait and `TemplateAgent` from `kowalski-core`, demonstrating robust composition for specialized agent creation.
*   **Tool Integration:** Seamlessly integrates `WebSearchTool` and `WebScrapeTool` from `kowalski-tools`, providing essential functionalities for web interaction.
*   **Descriptive System Prompt:** The system prompt is well-crafted, clearly instructing the LLM on its role, available tools, and expected response format for web-related tasks, emphasizing proactive tool use.

## 4. Weaknesses

*   **Hardcoded Tool Creation:** Similar to `kowalski-academic-agent` and `kowalski-code-agent`, tools are instantiated directly within the `WebAgent::new` function. This approach limits flexibility for customizing the agent's toolkit without direct code modification.
*   **Manually Managed Tool List in Prompt:** The list of available tools and their usage instructions are hardcoded within the agent's system prompt. This makes the prompt brittle and requires manual updates whenever tool specifications change, increasing maintenance overhead and potential for inconsistencies. Unlike the `DataAgent`, this agent does not dynamically generate its tool prompt.
*   **`TemplateAgent` Abstraction Leak:** The `WebAgent` relies on `TemplateAgent`, an internal component of `kowalski-core` that is not publicly exposed. This inconsistency can hinder external developers' understanding and ability to replicate or customize agents effectively.
*   **Limited Advanced Web Interaction:** While it can search and scrape, the agent's current capabilities do not extend to more complex web interactions like filling out forms, authentication, or interacting with dynamic web applications (though the `WebScrapeTool` does have recursive scraping, this is still passive).
*   **No Integrated Browser Control:** Unlike OpenClaw, there's no explicit browser control functionality that would allow the agent to "browse" the web in a more active, human-like manner.

## 5. Potential Improvements & Integration into Rebuild

To evolve `kowalski-web-agent` into a more powerful and flexible component of the new Kowalski framework:

*   **Configurable Tool Instantiation:** Implement a configuration-driven approach for instantiating and registering tools, allowing users to easily enable/disable web tools or specify default search providers without changing the agent's source code.
*   **Dynamic Tool Prompt Generation:** Adopt the dynamic prompt generation approach seen in `DataAgent` and integrate it with the proposed unified `ToolManager` (from `kowalski-core`). This would centralize tool definition and ensure consistency and maintainability across all agents.
*   **Refined Base Agent or Public Template:** As with other agents, address the `TemplateAgent` abstraction. Either make `TemplateAgent` public or refactor `WebAgent` to directly use `BaseAgent` with explicit tool management.
*   **Advanced Web Interaction Tools:** Develop or integrate tools for more complex web interactions, such as:
    *   **Form Filling/Submission:** Tools to identify and interact with web forms.
    *   **Authentication:** Handling logins and sessions.
    *   **Dynamic Content Interaction:** Using browser automation (e.g., via headless browser libraries) to interact with JavaScript-heavy applications.
*   **Dedicated Browser Control Tool:** Consider creating a dedicated `BrowserControlTool` that allows the agent to emulate human browsing behavior, enabling it to navigate, click elements, and extract information more flexibly. This would directly address a key strength of OpenClaw.
*   **Enhanced Security for Web Access:** If advanced browser control is implemented, robust sandboxing (e.g., Docker containers for browser instances) should be a priority to isolate potential risks from malicious websites.

These improvements would transform the `kowalski-web-agent` into a highly capable and adaptable web research and interaction agent, aligning with the "100x better" vision for the Kowalski framework.