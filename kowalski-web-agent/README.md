# Kowalski Web Agent

**Initial Proof of Concept Agent**

A specialized AI agent for web research and online information retrieval, built on the Kowalski framework. The Web Agent provides intelligent, conversational access to web search and web scraping, enabling users to find, summarize, and analyze online content.

## What is the Web Agent?

The Web Agent is an AI-powered assistant that combines large language models with web search and scraping tools. It helps users discover, extract, and summarize information from the internet, supporting research, fact-checking, and content synthesis.

### Core Capabilities

- **Web Search**: Query the internet using multiple search providers (DuckDuckGo, Google, Bing)
- **Web Scraping**: Extract content from web pages for further analysis
- **Summarization**: Generate human-readable summaries of web content
- **Conversational Q&A**: Ask follow-up questions and get iterative, context-aware answers
- **Role-based Summaries**: Tailor explanations for different audiences (e.g., technical, family-friendly)
- **Streaming AI Analysis**: Real-time, conversational web research

## What Does It Do?

- **Search**: Finds relevant web pages for a given query
- **Extract**: Scrapes and processes content from web pages
- **Summarize**: Provides concise, audience-tailored summaries of online information
- **Interactive Research**: Supports follow-up questions and iterative exploration
- **Conversation History**: Maintains context for multi-step research tasks

## Example Usage

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

## How Could It Be Extended?

- **Additional Search Providers**: Integrate with more APIs (Google, Bing, academic search, etc.)
- **Deeper Scraping**: Support for dynamic content, login-protected pages, or API endpoints
- **Content Classification**: Detect news, blogs, forums, or scientific content
- **Fact Checking**: Cross-reference multiple sources for verification
- **Sentiment and Topic Analysis**: Extract opinions, topics, and trends from web content
- **Automated Alerts**: Notify users of new or changing information on topics of interest
- **Integration**: Embed in chatbots, research assistants, or browser extensions

## Potential Benefits

### For Researchers
- **Faster Discovery**: Rapidly find and summarize relevant online information
- **Contextual Summaries**: Get explanations tailored to your audience or expertise
- **Iterative Exploration**: Ask follow-up questions and dig deeper

### For Developers
- **API-First**: Integrate web research into your own tools and workflows
- **Customizable**: Extend with new providers, scrapers, or analysis modules

### For Organizations
- **Knowledge Aggregation**: Gather and synthesize information from across the web
- **Monitoring**: Track topics, competitors, or trends in real time

---

## Example Output

The example successfully:
- Performs a web search and lists results with titles, URLs, and snippets
- Fetches and processes the content of a selected web page
- Generates a simplified summary for a non-technical audience
- Handles streaming responses and conversation context

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

---

**Note:** This is an initial proof of concept agent. Features, reliability, and coverage will expand in future versions.

--- 