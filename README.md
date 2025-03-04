# Ollama Agent in Rust

> "The best way to predict the future is to implement it." - Alan Kay
> "Rust is the language that makes you feel like a superhero while writing code." - Anonymous

This is a first attempt at creating a simple agent in Rust that communicates with the Ollama server. It's a work in progress and more features will be added in the future.

## Project Overview

This project implements a basic agent that can communicate with Ollama's API, supporting both regular chat and streaming responses. It's built as a learning exercise and foundation for more complex agent implementations.

> "Simplicity is prerequisite for reliability." - Edsger W. Dijkstra

## Features

- Basic chat functionality with Ollama
- Streaming response support
- Error handling
- Configurable base URL
- Simple and clean API

## Project Structure

```
.
├── src/
│   ├── main.rs         # Main entry point and example usage
│   └── agent.rs        # Agent implementation and types
├── Cargo.toml          # Project dependencies and configuration
└── README.md          # This file
```

> "The only way to do great work is to love what you do." - Steve Jobs

## Dependencies

- tokio: Async runtime
- reqwest: HTTP client
- serde: Serialization/deserialization
- serde_json: JSON handling

## Building and Running

1. Make sure you have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone this repository:
```bash
git clone <repository-url>
cd ollama-agent
```

3. Build the project:
```bash
cargo build
```

4. Run the example:
```bash
cargo run
```

> "The best way to learn a new programming language is by writing programs in it." - Dennis Ritchie

## Future Improvements

- [x] Add support for more Ollama API endpoints
- [x] Implement conversation history
- [x] Add configuration file support
- [ ] Implement proper streaming response parsing
- [ ] Add more error handling cases
- [ ] Add unit tests and integration tests
- [ ] Add documentation
- [ ] Add CLI interface

> "The only limit to our realization of tomorrow will be our doubts of today." - Franklin D. Roosevelt

## Contributing

Feel free to open issues or submit pull requests. This is a learning project, so any feedback or suggestions are welcome!

## License

This project is open source and available under the MIT License.

> "The best way to predict the future is to create it." - Peter Drucker 