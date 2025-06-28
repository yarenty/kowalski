# Kowalski Federation

🚧 **UNDER CONSTRUCTION / Work in Progress** 🚧

> **Note:** This module is in active development. Major design decisions—especially regarding multi-agent cooperation protocols (A2A, ACP, MCP, or custom)—are still to be made. Expect breaking changes and evolving APIs.

---

## Description

`kowalski-federation` aims to provide the foundation for multi-agent collaboration, orchestration, and communication in the Kowalski AI ecosystem. It is designed to enable distributed, federated, and cooperative agent systems, supporting a variety of agent roles (coordinator, worker, observer) and message-based protocols.

---

## Dependencies

- **kowalski-core** — Core agent abstractions, tools, and types
- **tokio** — Async runtime
- **serde** — Serialization/deserialization
- **serde_json** — JSON support
- **async-trait** — Async trait support
- **thiserror** — Error handling
- **tracing** — Structured logging and tracing
- **uuid** — Unique IDs for agents, messages, and tasks

---

## Architecture

```
kowalski-federation/
├── src/
│   ├── agent.rs         # FederatedAgent trait and federation roles
│   ├── error.rs         # FederationError types
│   ├── message.rs       # FederationMessage and message types
│   ├── orchestrator.rs  # Orchestrator for task delegation and coordination
│   ├── registry.rs      # AgentRegistry for membership and lookup
│   ├── lib.rs           # Library entry point
│   └── tests/           # Tests and protocol experiments
```

- **FederatedAgent**: Trait for agents participating in a federation (roles: coordinator, worker, observer)
- **Orchestrator**: Manages task delegation, assignment, and status
- **AgentRegistry**: Tracks agent membership and roles
- **FederationMessage**: Standardized message format for inter-agent communication

---

## Current & Planned Functionality

- **Agent registration and discovery**
- **Role assignment (coordinator, worker, observer)**
- **Task delegation and assignment**
- **Message passing and broadcasting**
- **Task status tracking and updates**
- **Basic error handling and reporting**

### Example: Creating and Registering a Federated Agent

```rust
use kowalski_federation::{FederatedAgent, FederationRole, AgentRegistry};
use std::sync::Arc;
use tokio::sync::RwLock;

let registry = Arc::new(AgentRegistry::new());
// let agent = ... // your FederatedAgent implementation
// registry.register_agent(Arc::new(RwLock::new(agent))).await?;
```

### Example: Sending a Federation Message

```rust
use kowalski_federation::{FederationMessage, MessageType};

let message = FederationMessage::new(
    MessageType::TaskDelegation,
    "coordinator".to_string(),
    Some("worker1".to_string()),
    "Task content here".to_string(),
    None,
);
// agent.send_message("worker1", message).await?;
```

---

## Open Questions & Design Decisions

- **Multi-agent protocol:** Should we use an existing protocol (A2A, ACP, MCP) or invent our own?
- **Task routing:** How should tasks be assigned—by capability, load, or other criteria?
- **Security & trust:** How do agents authenticate and authorize each other?
- **Scalability:** What are the bottlenecks for large federations?
- **Persistence:** Should agent state and task history be persisted?
- **Extensibility:** How can new agent types and protocols be plugged in?

---

## Future Enhancements

- Protocol selection and implementation (A2A, ACP, MCP, or custom)
- Advanced task routing and load balancing
- Secure agent authentication and encrypted messaging
- Persistent registry and orchestrator state
- Federation-wide logging and monitoring
- Support for agent federation across networks/clusters
- Integration with external orchestrators or workflow engines

---

**This module is a work in progress. Contributions, suggestions, and protocol discussions are welcome!** 