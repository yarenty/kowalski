# Changelog

> "History is written by the victors, changelogs are written by developers who broke something." - Winston Churchill (probably)

All notable changes to this project will be documented in this file, or at least we'll try to remember to do so.

## [0.5.0] - 2024-06-09

> "Version 0.5.0: The Great Kowalski Restructurization. Now with 100% more modules!"

### 🚀 Major Release: Modular Kowalski
- **Project Restructurization:**
  - Split the codebase into clear, separate modules:
    - `kowalski-core`: Foundational types, agent abstractions, conversation, roles, configuration, error handling, and toolchain logic.
    - `kowalski-agent-template`: Flexible agent base, builder, and ready-to-use templates for rapid agent development.
    - `kowalski-tools`: Pluggable tools for code, data, web, and document analysis.
    - `kowalski-federation`: (WIP) Multi-agent orchestration, registry, and federation protocols.
    - Specific agents (e.g., academic, code, data, web) now live in their own crates.
  - Each module now has its own README and clear documentation.
  - Lays the groundwork for future multi-agent, federated, and plugin-based development.

### 🏗️ Architecture
- **Separation of Concerns:** Each module is now responsible for a single aspect of the system, making the codebase easier to maintain, extend, and test.
- **Extensibility:** New agents, tools, and federation protocols can be added without touching the core logic.
- **Documentation:** All major modules now have comprehensive, modern README files.

### 🧪 Federation (Experimental)
- Initial implementation of `kowalski-federation` for multi-agent orchestration.
- Open questions remain about protocol selection (A2A, ACP, MCP, or custom).
- Marked as UNDER CONSTRUCTION—expect rapid changes and design discussions.

### 🧰 Tools
- All tools (code, data, web, document) are now in `kowalski-tools` and can be plugged into any agent.

### 🧑‍💻 Agent Templates
- `kowalski-agent-template` provides a builder and templates for fast custom agent creation.
- General and research agent templates included as examples.

### 🗃️ Other
- Updated and unified dependency management across all modules.
- Improved test coverage and modular test structure.
- Cleaned up legacy code, TODOs, and dead ends from previous versions.

---

## [0.3.0] - 2024-03-10

> "Version 0.3.0: Because 0.2.0 wasn't confusing enough." - A Version Control Enthusiast

### 🎭 Added
- **CLI Interface** (because typing commands is more fun than clicking buttons):
  - `kowalski chat`: Talk to your AI without all the fancy UI
  - `kowalski academic`: Analyze papers without actually reading them
  - `kowalski model`: Manage your AI models like a pro
  - Command-line arguments that make sense (for once)
  - Helpful error messages (they're still errors, but at least they're helpful)

### 🔧 Changed
- Completely revamped command-line interface (it's not just a bunch of flags anymore)
- Improved model management commands (your AI models are now properly domesticated)
- Enhanced error handling in CLI (because users deserve to know what they did wrong)
- Better streaming response handling (watch your AI think in real-time, now with better formatting)

### 🐛 Fixed
- CLI argument parsing issues (now it actually understands what you're trying to say)
- Model management command errors (your models won't disappear into the void anymore)
- Response streaming formatting (no more broken lines or missing characters)
- Various "it works on my machine" issues (it still might not work on yours, but at least we tried)

### 📚 Documentation
- Added CLI usage examples (because reading the code is so last year)
- Updated README with command-line instructions (they're actually useful this time)
- Added command help messages (they're sarcastic, but they work)
- Improved error messages (they're still errors, but at least they're funny)

### 🔬 Technical Debt
- Replaced quick CLI hacks with slightly more sophisticated CLI hacks
- Moved CLI-related TODOs to actual GitHub issues
- Pretended to understand command-line argument parsing better

### 🎯 Dependencies
- Added `clap` for proper CLI argument parsing (because parsing strings manually is so 2010)
- Updated other dependencies (because old code is like old milk - it smells bad)
- Removed deprecated dependencies (they served us well, but it's time to move on)

## [0.2.0] - 2024-03-09

> "The best time to write a changelog is when you make the changes. The second best time is right before a release when you've forgotten everything you did." - Ancient Developer Proverb


### 🎭 Added
- **New Agents** (because one AI personality wasn't enough):
  - `GeneralAgent`: Your friendly neighborhood AI with a dash of sass
  - `ToolingAgent`: The Swiss Army knife of web research
  - `AcademicAgent`: The one that actually reads the papers

- **Tool System** (like Batman's utility belt, but for AI):
  - Web browsing capabilities (because opening Chrome is too mainstream)
  - DuckDuckGo integration (Google who?)
  - HTML parsing with multiple fallback strategies
  - Dynamic content handling (JavaScript can't hide from us anymore)
  - Rate limiting (to avoid angry emails from server admins)

- **Examples** (because documentation is better with code):
  - `model_manager`: Herding your AI models like cats
  - `academic_research`: Making research papers readable again
  - `web_research`: Like having a very fast research assistant
  - `web_search`: For when typing in a browser is too much work
  - `web_dynamic`: Handling modern web apps like a pro
  - `web_static`: Old-school HTML scraping
  - `general_chat`: When you just want to chat with a sarcastic AI

### 🔧 Changed
- Completely revamped agent architecture (it's not spaghetti code anymore, we promise)
- Improved conversation management (your AI won't forget things... as often)
- Enhanced streaming responses (watch your AI think in real-time)
- Better error handling (because things will go wrong, we just handle it better now)

### 🐛 Fixed
- Memory leaks in conversation handling (your RAM can thank us later)
- Race conditions in async operations (time is now properly wibbly-wobbly)
- Various "it works on my machine" issues
- That one bug that nobody could reproduce but everyone complained about

### 📚 Documentation
- Added sarcastic comments throughout the codebase
- Created actually useful examples (a rare achievement)
- Updated README with proper setup instructions
- Added this CHANGELOG (because git log was getting boring)

### 🔬 Technical Debt
- Replaced quick hacks with slightly more sophisticated hacks
- Moved TODOs to actual GitHub issues
- Pretended to understand async/await better

### 🎯 Dependencies
- Updated all the things (except the ones that would break everything)
- Added more crates (because why solve problems yourself?)
- Removed deprecated dependencies (they served us well)

## [0.1.0] - 2024-03-07

### Added
- Initial release
- Basic Ollama integration
- Proof that we could make it work
- A lot of hopes and dreams



> "Change is inevitable, except from a vending machine." - Robert C. Gallagher

> "Version numbers are like birthdays - they keep increasing but nothing really changes." - A Cynical Developer

### Added
- Basic agent functionality (because talking to machines wasn't complicated enough)
- Multiple model support (because one AI model isn't confusing enough)
- Conversation management (like herding cats, but with more JSON)
- Role-based interactions (giving AI personalities, what could go wrong?)
- PDF and text file support (because copy-pasting was too mainstream)
- Streaming responses (watch your AI think in real-time, it's like watching paint dry but more expensive)
- Configuration system (because hardcoding values is too simple)
- Error handling (because we're optimists who plan for the worst)

### Features
- Implemented `Agent` struct (it's like a digital pet, but less cuddly)
- Added `ModelManager` for handling Ollama models (your personal AI zookeeper)
- Created `Role`, `Audience`, and `Preset` enums (because we love pretending our AI has a personality)
- Added `PdfReader` and `PaperCleaner` utilities (because PDFs are like onions - they have layers and make you cry)
- Implemented conversation history (because AIs need memories too)
- Added streaming support (for those who enjoy watching their CPU melt in real-time)

### Technical Debt
- "TODO" comments that will definitely be addressed in the next version (narrator: they won't)
- Some magic numbers that seemed like a good idea at 3 AM
- Documentation that assumes the reader can read minds
- Error messages that are more cryptic than your ex's texts

### Known Issues
- Sometimes the AI gets philosophical (we're working on reducing its exposure to existential literature)
- Configuration files multiply like rabbits in the wrong directory
- Error messages occasionally include Shakespeare quotes (we suspect the AI is going through a literature phase)
- The code works (this is suspicious and under investigation)

> "It's not a bug, it's an undocumented feature." - Anonymous  
> "The code is more what you'd call 'guidelines' than actual rules." - Pirates of the Caribbean, probably

### Dependencies
- Added every crate that looked interesting on crates.io
- Removed half of them because they were causing conflicts
- Added them back because the errors were worse
- Settled on a set that mostly works (fingers crossed)

### Documentation
- Added comments that range from "obviously redundant" to "cryptically useless"
- Created a README that nobody will read
- Added docstrings that are more entertaining than informative
- Included examples that work 60% of the time, every time

> "Documentation is like true love - it exists, but it's hard to find." - A Documentation Writer
> "The only thing worse than no documentation is wrong documentation." - A Frustrated Developer



[0.2.0]: https://github.com/yarenty/kowalski/releases/tag/0.2.0 
[0.1.0]: https://github.com/yarenty/kowalski/releases/tag/0.1.0 
