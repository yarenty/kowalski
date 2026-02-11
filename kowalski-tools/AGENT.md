# kowalski-tools: Pluggable Agent Tools

## 1. Purpose

The `kowalski-tools` crate is designed to provide a collection of pluggable tools that Kowalski agents can utilize to interact with the external environment. These tools enable agents to perform various tasks such as filesystem operations, data analysis (CSV), web searching, web scraping, and PDF processing. It acts as a repository for concrete implementations of the `Tool` trait defined in `kowalski-core`.

## 2. Structure

The `kowalski-tools/src` directory is organized by the type of functionality each tool provides:

*   **`code.rs`**: Intended for code analysis tools (e.g., `JavaAnalysisTool`, `PythonAnalysisTool`, `RustAnalysisTool`).
*   **`csv.rs`**: Contains the `CsvTool` for processing and analyzing CSV data, including statistical summaries.
*   **`document/`**: Houses document-related tools, specifically `pdf.rs` which provides the `PdfTool` for extracting content from PDF files.
*   **`fs.rs`**: Implements the `FsTool` for various filesystem operations like listing directories, finding files, and reading file contents.
*   **`web/`**: Contains web-interaction tools, including `scrape.rs` (for `WebScrapeTool`) and `search.rs` (for `WebSearchTool`).
*   **`tool.rs`**: Defines a `ToolManager` struct which aims to manage the registration and execution of tools.

## 3. Strengths

*   **Clear Tool Abstraction:** Tools are implemented using the `Tool` trait from `kowalski-core`, providing a consistent interface.
*   **Specialized Functionality:** Provides practical tools for common agent tasks (FS, CSV, Web, PDF).
*   **Modular Design:** Tools are logically grouped into modules (e.g., `web`, `document`), which aids organization.
*   **Extensibility:** The design allows for easy addition of new tools by implementing the `Tool` trait.
*   **Dynamic Tool Prompting (in DataAgent):** The `CsvTool` demonstrates the potential for dynamic generation of tool descriptions for LLM prompts, significantly improving flexibility.

## 4. Weaknesses

*   **Redundant Type Definitions:** The `kowalski-tools/src/lib.rs` file duplicates the definitions of `ToolParameter` and `ParameterType` that are already present in `kowalski-core/src/tools.rs`. This leads to unnecessary code duplication and potential for inconsistencies.
*   **Underutilized `ToolManager`:** A `ToolManager` exists in `tool.rs` which is a good concept for centralized tool handling, but it is not currently used by the `BaseAgent` in `kowalski-core` or the specialized agents. This means tool management logic is currently dispersed and less efficient.
*   **Incomplete `PdfTool`:** The `PdfTool`'s text and image extraction functionalities are noted as placeholders and are not fully implemented, limiting its practical utility.
*   **Hardcoded Tool List in Prompts (most agents):** With the exception of `DataAgent`, most specialized agents manually list tool descriptions and parameters in their system prompts, making them brittle and difficult to maintain when tool specifications change.
*   **Basic Task Dispatch:** The `execute` method within individual tools uses simple `match` statements on string-based task names, which can become cumbersome and less robust as the number of tasks per tool grows.

## 5. Potential Improvements & Integration into Rebuild

To enhance `kowalski-tools` and integrate it effectively into a more robust Kowalski framework:

*   **Eliminate Type Duplication:** Remove the redundant `ToolParameter` and `ParameterType` definitions from `kowalski-tools/src/lib.rs`, exclusively using the definitions from `kowalski-core`.
*   **Integrate `ToolManager` into Core:** Move the `ToolManager` concept (or a refined version of it) into `kowalski-core` and make it the central mechanism for `BaseAgent` and all specialized agents to register and execute tools. This would standardize tool management across the framework.
*   **Complete `PdfTool` Implementation:** Fully implement the text and image extraction functionalities of the `PdfTool` to make it a valuable resource for document analysis.
*   **Automated Tool Schema Generation:** Implement a mechanism within the unified `ToolManager` to automatically generate tool descriptions and parameter schemas (e.g., in OpenAPI JSON format) directly from the `Tool` trait implementation. This dynamic schema can then be provided to the LLM, eliminating manual prompt updates and making agents more adaptable.
*   **Refined Task Dispatch:** Consider more robust task dispatch mechanisms within tools, potentially using a trait for tasks or a declarative approach, to reduce the complexity of `match` statements for numerous tasks.
*   **Granular Tool Crates (Long-term):** As the framework scales, consider splitting highly complex tools (e.g., `code.rs` could become `kowalski-tool-rust-analyzer`, `kowalski-tool-java-analyzer`) into their own dedicated crates for better isolation, versioning, and independent development. This aligns with the "more granular" idea mentioned in the main `README.md`.

These improvements will make `kowalski-tools` a more consistent, flexible, and powerful component of the Kowalski ecosystem, supporting the goal of a "100x better" agent framework.