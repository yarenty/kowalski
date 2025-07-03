# Kowalski vs. LangChain: Performance Benchmark Plan

This document outlines a detailed plan for benchmarking the performance of the Kowalski agent framework (Rust) against a comparable LangChain implementation (Python). The goal is to quantitatively demonstrate Kowalski's advantages in terms of latency, throughput, and resource utilization for common agentic tasks.

---

## 1. Objectives

*   Measure and compare end-to-end latency for various agent tasks.
*   Measure and compare throughput (Requests Per Second - RPS) under increasing load.
*   Measure and compare CPU and memory utilization.
*   Identify performance bottlenecks in both frameworks.
*   Provide data-driven evidence for Kowalski's performance claims.

---

## 2. General Methodology

*   **Controlled Environment:** All benchmarks will be run on identical hardware with consistent network conditions.
*   **Identical LLM:** The *exact same LLM* (e.g., `llama3.2` via Ollama) will be used for both frameworks to isolate framework performance.
*   **Equivalent Agent Logic:** For each scenario, the agent logic (tool definitions, memory interaction, prompt engineering) will be implemented as equivalently as possible in both Rust (Kowalski) and Python (LangChain).
*   **Load Generation:** A dedicated load testing tool will be used to simulate concurrent requests.
*   **Metric Collection:** Automated tools will collect latency, throughput, CPU, and memory data.
*   **Statistical Rigor:** Each test will be run multiple times, and results will be averaged with standard deviations.

---

## 3. Benchmark Scenarios (High-Level Pseudo-Code)

For each scenario, we will implement the logic in both Kowalski (Rust) and LangChain (Python). The `main` function of each benchmark script will orchestrate the test.

### Scenario 1: Simple LLM Call (Baseline)

*   **Description:** A basic interaction with the LLM that does not involve tools or memory retrieval. Measures the fundamental overhead of the framework and LLM integration.
*   **Kowalski (Rust) Pseudo-Code:**
    ```rust
    // In kowalski-cli or a dedicated benchmark binary
    async fn benchmark_simple_llm_call(num_requests: usize) {
        let config = load_config();
        let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await.unwrap();
        let conversation_id = agent.start_conversation("llama3.2");

        for _ in 0..num_requests {
            let start_time = Instant::now();
            let response = agent.chat_with_history(&conversation_id, "Tell me a short joke.", None).await.unwrap();
            // Process stream to get full response
            let _full_response = process_stream_response(response).await;
            let end_time = Instant::now();
            record_latency(start_time.elapsed());
        }
    }
    ```
*   **LangChain (Python) Pseudo-Code:**
    ```python
    # In a dedicated Python benchmark script
    from langchain_community.llms import Ollama
    from langchain_core.prompts import ChatPromptTemplate

    def benchmark_simple_llm_call(num_requests):
        llm = Ollama(model="llama3.2")
        prompt = ChatPromptTemplate.from_messages([("user", "{input}")])
        chain = prompt | llm

        for _ in range(num_requests):
            start_time = time.time()
            response = chain.invoke({"input": "Tell me a short joke."})
            end_time = time.time()
            record_latency(end_time - start_time)
    ```

### Scenario 2: Single Tool Use (e.g., Weather Tool)

*   **Description:** An agent task requiring a single external tool call. Measures tool invocation overhead and data passing.
*   **Kowalski (Rust) Pseudo-Code:**
    ```rust
    // Assume a `weather_tool` is registered with the agent
    async fn benchmark_single_tool_use(num_requests: usize) {
        let config = load_config();
        let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await.unwrap();
        let conversation_id = agent.start_conversation("llama3.2");

        for _ in 0..num_requests {
            let start_time = Instant::now();
            // Agent's internal logic would call chat_with_tools, which then calls execute_tool
            let _response = agent.chat_with_tools(&conversation_id, "What's the weather in London?").await.unwrap();
            let end_time = Instant::now();
            record_latency(start_time.elapsed());
        }
    }
    ```
*   **LangChain (Python) Pseudo-Code:**
    ```python
    # Assume a `WeatherTool` is defined and integrated into an agent
    from langchain.agents import AgentExecutor, create_react_agent
    from langchain_core.tools import Tool

    def get_weather(location: str) -> str:
        # Simulate API call
        return f"The weather in {location} is sunny."

    tools = [Tool(name="weather", func=get_weather, description="...")]

    def benchmark_single_tool_use(num_requests):
        llm = Ollama(model="llama3.2")
        # ... setup agent with tools ...
        agent_executor = AgentExecutor(...)

        for _ in range(num_requests):
            start_time = time.time()
            response = agent_executor.invoke({"input": "What's the weather in London?"})
            end_time = time.time()
            record_latency(end_time - start_time)
    ```

### Scenario 3: Memory Retrieval (RAG)

*   **Description:** An agent task requiring retrieval of relevant context from a vector store (semantic memory) before generating a response. Measures the efficiency of the RAG pipeline.
*   **Kowalski (Rust) Pseudo-Code:**
    ```rust
    // Assume semantic_memory is populated with some data
    async fn benchmark_memory_retrieval(num_requests: usize) {
        let config = load_config();
        let mut agent = BaseAgent::new(config, "BenchmarkAgent", "").await.unwrap();
        let conversation_id = agent.start_conversation("llama3.2");

        // Pre-populate semantic memory for testing
        agent.semantic_memory.add(MemoryUnit { /* ... */ }).await.unwrap();

        for _ in 0..num_requests {
            let start_time = Instant::now();
            // chat_with_history will internally call semantic_memory.retrieve
            let response = agent.chat_with_history(&conversation_id, "Tell me about the project Kowalski.", None).await.unwrap();
            let _full_response = process_stream_response(response).await;
            let end_time = Instant::now();
            record_latency(start_time.elapsed());
        }
    }
    ```
*   **LangChain (Python) Pseudo-Code:**
    ```python
    # Assume a vector store (e.g., Chroma, FAISS) is set up and populated
    from langchain_community.vectorstores import Chroma
    from langchain_community.embeddings import OllamaEmbeddings
    from langchain.chains import RetrievalQA

    def benchmark_memory_retrieval(num_requests):
        embeddings = OllamaEmbeddings(model="llama3.2")
        vectorstore = Chroma(embedding_function=embeddings, ...)
        # ... populate vectorstore ...

        llm = Ollama(model="llama3.2")
        qa_chain = RetrievalQA.from_chain_type(llm=llm, retriever=vectorstore.as_retriever())

        for _ in range(num_requests):
            start_time = time.time()
            response = qa_chain.invoke({"query": "Tell me about the project Kowalski."})
            end_time = time.time()
            record_latency(end_time - start_time)
    ```

### Scenario 4: Data Processing (CSV Analysis)

*   **Description:** Using the `kowalski-data-agent` (or equivalent in LangChain) to process a CSV file and extract insights. This will involve file I/O, data parsing, and potentially statistical calculations.
*   **Kowalski (Rust) Pseudo-Code:**
    ```rust
    // In kowalski-data-agent benchmark
    async fn benchmark_csv_analysis(num_requests: usize, csv_data: &str) {
        let config = load_config();
        let mut data_agent = DataAgent::new(config).await.unwrap();
        let conversation_id = data_agent.start_conversation("llama3.2");

        for _ in 0..num_requests {
            let start_time = Instant::now();
            let analysis_result = data_agent.process_csv(csv_data).await.unwrap();
            let analysis_prompt = format!("Analyze this data: {}", serde_json::to_string(&analysis_result).unwrap());
            let response = data_agent.chat_with_history(&conversation_id, &analysis_prompt, None).await.unwrap();
            let _full_response = process_stream_response(response).await;
            let end_time = Instant::now();
            record_latency(start_time.elapsed());
        }
    }
    ```
*   **LangChain (Python) Pseudo-Code:**
    ```python
    # Using LangChain's CSV agent or similar custom tool
    from langchain_experimental.agents import create_csv_agent
    import pandas as pd

    def benchmark_csv_analysis(num_requests, csv_file_path):
        llm = Ollama(model="llama3.2")
        # This might involve creating a custom tool for CSV processing
        agent = create_csv_agent(llm, csv_file_path, verbose=False)

        for _ in range(num_requests):
            start_time = time.time()
            response = agent.invoke({"input": "Analyze the data and provide key insights."})
            end_time = time.time()
            record_latency(end_time - start_time)
    ```

---

## 4. Implementation Details

### Load Generation

*   **Rust:** For simple HTTP-based benchmarks, `wrk` or a custom `tokio`-based client can be used. For more complex agent interactions, a dedicated Rust client that calls the agent's API directly will be developed.
*   **Python:** `locust` or a custom `asyncio`-based client will be used.

### Metric Collection

*   **Latency:** Measured directly within the benchmark scripts using `Instant::now()` (Rust) and `time.time()` (Python).
*   **Throughput:** Calculated from total requests and total time.
*   **CPU/Memory:**
    *   **Linux:** `perf`, `htop`, `psutil` (Python), or `sysinfo` crate (Rust) for programmatic access.
    *   **macOS:** `Instruments` or `top`/`htop`.
    *   Monitoring will be done externally to avoid impacting the benchmarked process.

### Reporting

*   Results will be stored in CSV or JSON format.
*   Data visualization using Python libraries (Matplotlib, Seaborn) or dedicated benchmarking tools.

---

## 5. Next Steps

1.  **Define Specific Inputs:** For each scenario, create concrete input data (e.g., specific jokes, a standardized CSV file, a set of memory units).
2.  **Implement Benchmark Scripts:** Write the full, runnable code for each scenario in both Kowalski and LangChain.
3.  **Set Up Test Environment:** Ensure Ollama is running with `llama3.2` and Qdrant is configured for memory benchmarks.
4.  **Execute Benchmarks:** Run the tests under varying load conditions.
5.  **Analyze and Report:** Collect, process, and visualize the results.

This plan provides a solid foundation for a rigorous performance comparison. We can iterate on the pseudo-code to refine the exact implementation details for each scenario.