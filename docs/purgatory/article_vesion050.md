# Kowalski: The Rust-native Agentic AI Framework Evolves to v0.5.0

I’m excited to share the latest milestone for **Kowalski**—a powerful, modular agentic AI framework built in **Rust** for local-first, extensible LLM workflows. Three months ago, I released **Kowalski v0.2.0**, a major stepping stone, where I start playing with different tools. Today, the codebase has evolved dramatically, with **v0.5.0** rolling out extensive refactoring, architectural improvements, and a _wealth_ of new functionality ;-).



---

**TL;DR:** Kowalski v0.5.0 brings deep refactoring, modular architecture, multi-agent orchestration, and robust docs across submodules. If you care about Rust, AI agents, and extensible tooling, now’s the time to jump in and build together!

---


## A Deep Dive into v0.5.0

Since v0.2.0, the Kowalski ecosystem has undergone:

* **Massive refactoring of core abstractions and crate structure**:
  The **kowalski-core**, **kowalski-tools**, and agent-specific crates (academic, code, data, web) have each been reorganized into clean, self-contained modules with dedicated `README.md` files, detailing usage, examples, and extension points ([github.com/yarenty/kowalski][1]).

* **New federation layer for multi-agent orchestration**:
  The emerging **kowalski-federation** crate introduces a flexible registry and task-passing layers, enabling future multi-agent workflows and scalable core collaboration.

* **Improved CLI & agent-specific binaries**:
  Each agent—academic, code, data, web—comes with its own improved CLI and documentation. The **kowalski-cli** now supports seamless interaction across all binaries, with better streaming, configurable prompts, and embedded tool sets.

* **Enhanced pluggable tools**:
  The **kowalski-tools** crate now offers more granular support for CSV analysis, multi-language code analysis (Rust, Python, Java), web scraping, PDF/document parsing, and dynamic prompt strategies—each documented in submodule `README.md` files ([github.com][1]).

* **Rust API stability**:
  The core API, based on the `BaseAgent`, now supports typed configs, async multi-tool support, and more robust error handling, making embedding into larger Rust stacks smoother and more reliable.
  
![rust](https://www.rust-lang.org/static/images/rust-logo-blk.svg)


## Why Kowalski v0.5.0 Matters

Rust lovers and AI developers, here’s why this release stands out:

![architecture](img/architecture_v01.png)



**Full-stack Rust agentic workflows**
With zero Python dependencies, Kowalski compiles into performant, standalone binaries. Whether launching `kowalski-code-agent` for code reviews or embedding agents via the Rust API, you’re operating at native speed.

**Modular by design**
Each submodule is self-documented and self-contained, lowering the barrier for new contributors. Want to create a `PDFPresentationAgent` or integrate telemetry? Just read the README in the existing agent templates and go.

**Streamlined CLI experience**
The unified CLI gives consistent interfaces across agents. Under the hood, agents share core abstractions, so switching from data analysis to web scraping is seamless.

**Future-proof federation support**
The new federation crate opens the door to lightweight orchestrated, multi-agent workflows—think pipeline automations, task delegation, and agent-to-agent communication.

### Get Involved: Let’s Shape Agentic Rust Together

Here’s how you can partner with the project:

* **Extend the toolset**: add new agents (e.g., `document-summaries`, `intent-classification`), implement new tools, or polish existing ones.
* **Improve federation workflows**: help standardize protocols, design multi-agent orchestration logic, data passing, and telemetry.
* **Embed Kowalski in Rust services**: build bots, backend services, UI apps that leverage Kowalski agents for intelligent behavior.
* **Document and promote**: each submodule already includes README files—help expand examples, write blog posts, or record demos.
* **Contribute core enhancements**: testing, error handling, performance improvements in the `core` or `tools` crates.

## Start Using v0.5.0 Today

1. **Clone the repo**:

   ```bash
   git clone https://github.com/yarenty/kowalski.git
   cd kowalski
   ```
2. **Browse submodules & READMEs**: Each agent and tool lives in its own folder with clear instructions.
3. **Build & run**:

   ```bash
   cargo build --release
   ```
4. **Run agents**:

   ```bash
   ollama serve &
   ollama pull llama3.2
   ./target/release/kowalski-cli chat "Hey Kowalski, what's up?"
   ./target/release/kowalski-code-agent --file src/main.rs
   ```
5. **Embed in Rust**:

   ```rust
   use kowalski_core::{Config, BaseAgent};
   let mut agent = BaseAgent::new(Config::default(), "Demo", "Agent v0.5.0").await?;
   let conv = agent.start_conversation("llama3.2");
   agent.add_message(&conv, "user", "Summarize this code module").await?;
   ```

## Let’s Connect & Collaborate

If you’re as passionate about **Agentic AI** and **Rust** as I am, let’s talk 🚀. Whether you’d like to:

* Build new agents or tool integrations,
* Architect fully orchestrated agent systems,
* Demo Kowalski in your workflows,
* Co-author articles or demos in the Rust+AI space—

I’m ready to brainstorm on a call, pair on code, or publish together. Reach out via GitHub issues, PRs, or drop me a message to get started.

---

[1]: https://github.com/yarenty/kowalski"yarenty/kowalski: High performance Rust based AI Agent framework.- GitHub"
