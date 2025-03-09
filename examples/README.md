# 🤖 Kowalski's Playground of AI Shenanigans

> "I have not failed. I've just found 10,000 ways that won't work with LLMs." 
> - Thomas Edison (probably, if he was alive and trying to work with AI)

Welcome to the examples directory, where we demonstrate how to make AI do our bidding while pretending we know what we're doing!

> "To err is human, to really mess things up you need an AI agent."
> - Ancient Programming Proverb (circa 2024)

## 🎭 The Cast of Characters

### 1. `model_manager`
```bash
cargo run --example model_manager
```
Manages your Ollama models like a helicopter parent. Lists them, downloads them, and occasionally judges their life choices.

Features:
- Lists available models (and silently judges their size)
- Downloads models (slower than your first dial-up connection)
- Shows progress bars (because we're fancy like that)

### 2. `academic_research`
```bash
cargo run --example academic_research
```
Turns academic papers into something you might actually understand. It's like having a very smart friend who actually reads the papers instead of just the abstract.

Features:
- PDF processing (because PDFs are evil and need to be tamed)
- Scientific paper analysis (or as we call it, "fancy word interpretation")
- Interactive Q&A (like Stack Overflow, but it actually answers your question)

### 3. `web_research`
```bash
cargo run --example web_research
```
The digital equivalent of "let me Google that for you," but with more sophistication and slightly less sass.

Features:
- Web searching (because typing in a browser is so 2023)
- Content processing (turns web spaghetti into actual information)
- Summary generation (for when reading is too much work)

### 4. `web_search`
```bash
cargo run --example web_search
```
Like having a personal librarian who's really into the internet. Searches the web while you pretend to be productive.

Features:
- Uses DuckDuckGo (because we respect privacy... and Google's API pricing)
- Formats results nicely (we're not savages)
- Finds things you could have found yourself (but faster)

### 5. `web_dynamic`
```bash
cargo run --example web_dynamic
```
Handles JavaScript-heavy websites like a pro. It's like having a web browser that actually does what you want it to do.

Features:
- Processes social media pages (so you don't have to)
- Extracts content from dynamic sites (magic, basically)
- Collects metadata (because someone might care about that)

### 6. `web_static`
```bash
cargo run --example web_static
```
For when you just want to scrape some good old-fashioned HTML. No JavaScript drama here.

Features:
- Processes multiple URLs in parallel (because waiting is for the weak)
- Extracts structured content (turns chaos into... less chaos)
- Shows content previews (so you know it's working... mostly)

## 🛠 Configuration

Before running these examples, you'll need a `config.toml` file. Here's a template:

```toml
[ollama]
base_url = "http://localhost:11434"  # Unless your Ollama is running on Mars

[search]
provider = "duckduckgo"  # Because Google doesn't return our calls anymore
api_key = ""  # Optional, like your commitment to documentation

[chat]
temperature = 0.7  # How spicy you want your AI responses
max_tokens = 2048  # How chatty you want your AI to be
```

## 📋 Requirements

- Rust 1.75 or later (because we're living in the future)
- Running Ollama instance (preferably awake and cooperative)
- Internet connection (carrier pigeons not supported yet)
- `env_logger` for logging (because print statements are so 1990s, set `RUST_LOG=debug` for verbose output))
- A sense of humor (critical requirement)

## 🚀 Running Examples

To run any example with debug logging (and see all the chaos unfold):
```bash
RUST_LOG=debug cargo run --example <name>
```

> "In theory, there is no difference between theory and practice. In practice, there is."
> - Yogi Berra

## ⚠️ Warning

These examples may cause:
- Excessive productivity
- Reduced Stack Overflow dependency
- Uncontrollable urge to automate everything
- Sudden appreciation for AI's ability to misunderstand simple tasks
- Spontaneous bouts of debugging

> "I asked AI to make me a sandwich, and it spent 3 hours analyzing the etymology of 'sandwich'"
> - Anonymous Developer

Remember: With great power comes great responsibility to not break the internet while scraping it.

Happy coding! 🎉

*P.S. If something breaks, try turning it off and on again. If that doesn't work, check if your AI is having an existential crisis.* 