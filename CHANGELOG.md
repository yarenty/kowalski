# Changelog

> "Change is inevitable, except from a vending machine." - Robert C. Gallagher

All notable changes to this project will be documented in this file, because apparently, we need to keep track of our mistakes.

> "Version numbers are like birthdays - they keep increasing but nothing really changes." - A Cynical Developer

## [0.1.0] - 2024-03-17

> "The first version is like your first love - exciting but probably not the best." - A Romantic Programmer  
> "If debugging is the process of removing bugs, then programming must be the process of adding them." - Edsger W. Dijkstra

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

[0.1.0]: https://github.com/yarenty/kowalski/releases/tag/v0.1.0 