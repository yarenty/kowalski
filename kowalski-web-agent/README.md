# Kowalski Web Agent

**Initial Proof of Concept Agent**

A specialized AI agent for web research and online information retrieval, built on the Kowalski framework. The Web Agent provides intelligent, conversational access to web search and web scraping, enabling users to find, summarize, and analyze online content.

## What is the Web Agent?

The Web Agent is an AI-powered assistant that combines large language models with web search and scraping tools. It helps users discover, extract, and summarize information from the internet, supporting research, fact-checking, and content synthesis.

### Core Capabilities

- **Web Search**: Query the internet using multiple search providers (DuckDuckGo, Google, Bing)
- **Web Scraping**: Extract content from web pages for further analysis
- **ReAct-Style Tool Calling**: Intelligent tool usage with reasoning and iteration
- **Summarization**: Generate human-readable summaries of web content
- **Conversational Q&A**: Ask follow-up questions and get iterative, context-aware answers
- **Role-based Summaries**: Tailor explanations for different audiences (e.g., technical, family-friendly)
- **Streaming AI Analysis**: Real-time, conversational web research

## What Does It Do?

- **Search**: Finds relevant web pages for a given query
- **Extract**: Scrapes and processes content from web pages
- **Reason & Act**: Uses ReAct-style tool calling to intelligently chain tools
- **Summarize**: Provides concise, audience-tailored summaries of online information
- **Interactive Research**: Supports follow-up questions and iterative exploration
- **Conversation History**: Maintains context for multi-step research tasks

## ReAct-Style Tool Calling

The Web Agent implements a ReAct (Reasoning and Acting) loop that enables intelligent tool usage:

1. **Reason**: The agent analyzes the user's query and decides which tools to use
2. **Act**: The agent executes the appropriate tool (web_search or web_scrape)
3. **Observe**: The agent processes the tool's results
4. **Iterate**: The agent continues the loop until a final answer is reached

### Tool Usage Examples

The agent can intelligently chain tools:

```rust
// The agent will automatically:
// 1. Use web_search to find relevant URLs
// 2. Use web_scrape to extract content from promising URLs
// 3. Synthesize the information into a comprehensive answer
let response = agent.chat_with_tools(&conv_id, "What are the latest developments in AI?").await?;
```

## Example Usage

### Basic Usage

```rust
use kowalski_web_agent::WebAgent;
use kowalski_core::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let mut web_agent = WebAgent::new(config).await?;

    // Perform a web search
    let results = web_agent.search("AI").await?;
    for result in &results {
        println!("Title: {}", result.title);
        println!("URL: {}", result.url);
        println!("Snippet: {}", result.snippet);
    }

    // Fetch and summarize a web page
    if let Some(first) = results.first() {
        let page = web_agent.fetch_page(&first.url).await?;
        println!("Page Title: {}", page.title);
        println!("Content: {}", &page.content[..100]);
    }

    Ok(())
}
```

### Advanced Tool-Calling Usage

```rust
use kowalski_web_agent::WebAgent;
use kowalski_core::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let mut agent = WebAgent::new(config).await?;
    
    // Start a conversation
    let conv_id = agent.start_conversation("llama3.2");
    
    // Use ReAct-style tool calling for complex queries
    let response = agent.chat_with_tools(&conv_id, "What's the latest news about AI?").await?;
    println!("Response: {}", response);
    
    // The agent will automatically:
    // 1. Search for recent AI news
    // 2. Scrape relevant articles
    // 3. Synthesize the information
    // 4. Provide a comprehensive answer
    
    Ok(())
}
```

## How Could It Be Extended?

- **Additional Search Providers**: Integrate with more APIs (Google, Bing, academic search, etc.)
- **Deeper Scraping**: Support for dynamic content, login-protected pages, or API endpoints
- **Content Classification**: Detect news, blogs, forums, or scientific content
- **Fact Checking**: Cross-reference multiple sources for verification
- **Sentiment and Topic Analysis**: Extract opinions, topics, and trends from web content
- **Automated Alerts**: Notify users of new or changing information on topics of interest
- **Integration**: Embed in chatbots, research assistants, or browser extensions
- **Advanced Tool Chaining**: More sophisticated reasoning and tool selection logic

## Potential Benefits

### For Researchers
- **Faster Discovery**: Rapidly find and summarize relevant online information
- **Contextual Summaries**: Get explanations tailored to your audience or expertise
- **Iterative Exploration**: Ask follow-up questions and dig deeper
- **Intelligent Tool Usage**: The agent automatically chooses the right tools for each task

### For Developers
- **API-First**: Integrate web research into your own tools and workflows
- **Customizable**: Extend with new providers, scrapers, or analysis modules
- **ReAct Integration**: Leverage the reasoning and acting capabilities in your applications

### For Organizations
- **Knowledge Aggregation**: Gather and synthesize information from across the web
- **Monitoring**: Track topics, competitors, or trends in real time
- **Automated Research**: Reduce manual effort in information gathering and analysis

---

## Example Output

The example successfully:
- Performs a web search and lists results with titles, URLs, and snippets
- Fetches and processes the content of a selected web page
- Generates a simplified summary for a non-technical audience
- Handles streaming responses and conversation context
- Uses ReAct-style tool calling for intelligent information gathering

Running the web research example:

```
🤖 Starting web agent...
Web Agent Conversation ID: 12345678-90ab-cdef-1234-567890abcdef

🔍 Searching: AI

📑 Result:
Title: Artificial Intelligence - Wikipedia
URL: https://en.wikipedia.org/wiki/Artificial_intelligence
Snippet: Artificial intelligence (AI) is intelligence demonstrated by machines, in contrast to the natural intelligence displayed by humans and animals...

📑 Result:
Title: What is AI? | IBM
URL: https://www.ibm.com/topics/artificial-intelligence
Snippet: Artificial intelligence leverages computers and machines to mimic the problem-solving and decision-making capabilities of the human mind...

🌐 Processing first result: https://en.wikipedia.org/wiki/Artificial_intelligence

📝 Generating summary...
Artificial intelligence (AI) is when computers or machines are designed to think and learn like humans. It helps solve problems, make decisions, and can be found in things like voice assistants or self-driving cars.

✅ Summary complete!
```

Running the tool-calling demo:

```
Kowalski Web Agent Tool-Calling Demo
=====================================
Conversation started with ID: abc123

--- Query: What's the latest news about AI? ---
Processing with ReAct-style tool calling...
[DEBUG] Iteration 1: What's the latest news about AI?
[DEBUG] Tool call detected: web_search with input: latest AI news 2024
[DEBUG] Tool web_search executed successfully: [search results...]
[DEBUG] Iteration 2: Based on the tool result: [search results...]
[DEBUG] Tool call detected: web_scrape with input: https://example-news-site.com
[DEBUG] Tool web_scrape executed successfully: [article content...]
Final Response: Based on recent news, AI developments include...

--- End Query ---
```

---

**Note:** This is an initial proof of concept agent. Features, reliability, and coverage will expand in future versions.

--- 