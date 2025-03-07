# Kowalski

> "AI agents are like pets - they're cute but they make a mess." - Anonymous AI Developer
> "Programming is like writing a love letter to a computer that doesn't love you back." - Unknown

A Rust-based agent for interacting with Ollama models. Because apparently, we need another way to talk to AI.

## Project Overview

This project implements a basic agent that can communicate with Ollama's API, supporting both regular chat and streaming responses. It's built as a learning exercise and foundation for more complex agent implementations.

> "Simplicity is prerequisite for reliability." - Edsger W. Dijkstra

## Features

> "Features are like promises - they're great until you try to use them." - A Disappointed User

- 🤖 **Multiple Model Support**: Because one AI model is never enough
- 💬 **Conversation Management**: Keep track of your AI's ramblings
- 🎭 **Role-Based Interactions**: Give your AI a personality (or at least pretend to)
- 📝 **PDF and Text File Support**: Read files because typing is too mainstream
- 🔄 **Streaming Responses**: Watch your AI think in real-time (it's more exciting than it sounds)
- ⚙️ **Configurable Settings**: Customize everything until it breaks

## Installation

> "Installation is like cooking - it's easy until you burn something." - A Frustrated Developer

1. Clone the repository (because copying files manually is so last year):
   ```bash
   git clone https://github.com/yarenty/kowalski.git
   cd kowalski
   ```

2. Build the project (and pray it works):
   ```bash
   cargo build --release
   ```

3. Run the agent (and hope for the best):
   ```bash
   cargo run --release
   ```

## Usage

> "Usage instructions are like recipes - nobody reads them until something goes wrong." - A Support Agent

### Basic Usage

```rust
use kowalski::{Agent, Config};

// Create an agent (it's like hiring an assistant, but cheaper)
let config = Config::load()?;
let agent = Agent::new(config)?;

// Start a conversation (and hope it doesn't get weird)
let conversation_id = agent.start_conversation("mistral-small");

// Chat with the agent (it's like texting, but with more existential dread)
let response = agent.chat_with_history(&conversation_id, "Hello, how are you?", None).await?;
```

### Role-Based Interactions

> "Roles are like costumes - they make everything more interesting until someone takes them off." - A Theater Director

```rust
use kowalski::role::{Role, Audience, Preset};

// Create a translator role (because Google Translate is too mainstream)
let role = Role::translator(Some(Audience::Scientist), Some(Preset::Questions));

// Chat with the role (it's like having a conversation with someone who's pretending to be someone else)
let response = agent.chat_with_history(&conversation_id, "Translate this", Some(role)).await?;
```

### File Input

> "File input is like reading books - it's good for you but nobody does it." - A Librarian

```rust
use kowalski::utils::{PdfReader, PaperCleaner};

// Read from a PDF (because paper is so last century)
let content = PdfReader::read_pdf("document.pdf")?;

// Clean the content (because AI needs clean data, just like we need clean clothes)
let cleaned_content = PaperCleaner::clean(&content)?;

// Chat with the content (it's like having a book club, but with AI)
let response = agent.chat_with_history(&conversation_id, &cleaned_content, None).await?;
```

## Configuration

> "Configuration is like a relationship - it's complicated until you give up." - A System Administrator

The agent can be configured using a TOML file or environment variables:

```toml
[ollama]
base_url = "http://localhost:11434"
default_model = "mistral-small"

[chat]
temperature = 0.7
max_tokens = 512
stream = true
```

## Contributing

> "Contributing is like dating - it's fun until someone suggests changes." - An Open Source Maintainer

Contributions are welcome! Please feel free to submit a Pull Request. Just remember:
- Keep it clean (unlike my code)
- Add tests (because we all love writing tests)
- Update documentation (because reading code is so last year)

## License

> "Licenses are like prenuptial agreements - they're boring until you need them." - A Lawyer

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

> "Acknowledgments are like thank you notes - they're nice but nobody reads them." - A Grateful Developer

- Thanks to the Ollama team for making this possible
- Thanks to all contributors who helped make this project better
- Thanks to my coffee machine for keeping me awake during development 