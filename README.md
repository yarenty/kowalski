# Kowalski

> "AI agents are like pets – they're cute, but they make a mess."  
> "The future is modular, and so is Kowalski. Want a feature? Open an issue or submit a PR!"


A sophisticated Rust-based multi-agent framework for interacting with various LLM providers, with built-in support for federation, secure multi-party computation, and extensible tooling architecture.

---


## 🌟 Vision & Architecture
Kowalski is designed as a foundational framework for building intelligent, distributed agent systems that can collaborate securely and efficiently. The architecture supports both standalone operation and federated deployments with advanced privacy-preserving capabilities.

![Architecture](docs/img/architecture_v01.png)





```
kowalski/
├── kowalski-core/           # Core agent abstractions, conversation, roles, config, templates
├── kowalski-cli/            # Command-line interface
├── migrations/
│   └── legacy_prompts/      # Stashed prompts from legacy specialized agents
├── resources/               # Configs, tokenizer, etc.
└── ...                      # Examples, docs, etc.
```

---

## 📦 Module Overview

### **kowalski-core**
- Foundational types, agent abstractions, conversation, roles, configuration, error handling, toolchain logic.
- Includes `TemplateAgent` for building configurable agents.
- Designed for extensibility and async-first operation.
- [See details](./kowalski-core/README.md)

### **kowalski-cli**
- The main command-line interface for Kowalski.
- Provides a unified REPL and command set to interact with the core agent framework.

---

## 🚀 Installation & Setup

> "Installation is like cooking – it's easy until you burn something." – A Frustrated Developer

### 1. Prerequisites

- Rust (latest stable, install via [rustup.rs](https://rustup.rs))
- [Ollama](https://ollama.com/) (for local LLMs, e.g., llama3.2)
- (Optional) Other LLM providers (OpenAI, etc.)

### 2. Clone & Build

```bash
git clone https://github.com/yarenty/kowalski.git
cd kowalski
cargo build --release
```

### 3. Install & Run Ollama

```bash
# Install Ollama (see https://ollama.com/download)
ollama serve &

# Download a model (llama3.2 runs on CPU)
ollama pull llama3.2
```

### 4. Run Kowalski

```bash
cargo run --release --bin kowalski-cli
# Or use the CLI directly after building
./target/release/kowalski chat "Hello, world!"
```

---

## 🛠️ Usage

### CLI Examples

```bash
# Chat with an LLM
kowalski chat "What's the best way to learn Rust?"

# Analyze a PDF
kowalski academic --file research.pdf

# Web search
kowalski tool search "rust async programming"

# Code analysis
kowalski tool code ./src/main.rs
```

### Rust API Example

```rust
use kowalski_core::{Agent, BaseAgent, Config};
let config = Config::default();
let mut agent = BaseAgent::new(config, "Demo Agent", "A test agent").await?;
let conv_id = agent.start_conversation("llama3.2");
agent.add_message(&conv_id, "user", "Hello, world!").await;
```

---

## 🤖 Existing Agents & How to Run

Kowalski formerly utilized dedicated agent crates (`kowalski-web-agent`, `kowalski-code-agent`, etc.). These have been unified into a single `TemplateAgent` located in `kowalski-core`.

To run specific agent functional "personas" (like `web` or `code`), you can supply the persona name to the CLI, which will load the respective system prompt and configuration dynamically:

```bash
cargo run --release --bin kowalski-cli
kowalski> create code
kowalski> chat code-agent
```

Legacy prompt configurations are currently stored in `migrations/legacy_prompts/`.

---

## 📖 Documentation & Links

- [CHANGELOG.md](./CHANGELOG.md)
- [ROADMAP.md](./ROADMAP.md)
- [Each module's README](./kowalski-core/README.md), etc.

---

## 🤝 Contributing

> "Contributing is like dating – it's fun until someone suggests changes." – An Open Source Maintainer

- PRs, issues, and feature requests are welcome!
- Please add tests and update docs.
- See [CONTRIBUTING.md](./CONTRIBUTING.md) if available.

---

## 📝 License

> "Licenses are like prenuptial agreements – they're boring until you need them." – A Lawyer

MIT License. See [LICENSE](./LICENSE).

---

## 🙏 Acknowledgments

> "Acknowledgments are like thank you notes – they're nice but nobody reads them." – A Grateful Developer

- Thanks to the Ollama team and all open source contributors.
- Thanks to my coffee machine for keeping me awake during development.
- Thanks to everyone who opens an issue, even if it's just to say "it doesn't work".

---

## 📈 Activity

![Alt](https://repobeats.axiom.co/api/embed/7ac42f1d632566d6dbc38b23cbdcd8c1881b3856.svg "Repobeats analytics image")

---

**For the latest features, roadmap, and future plans, see [ROADMAP.md](./ROADMAP.md).**