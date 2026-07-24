# kowalski-mcp-base

Shared framework for **first-party Kowalski MCP servers** (tool *source 1*).

Integrates:

- **Transport** — [`McpHandler`](src/transport.rs) + stateless Streamable HTTP + stdio (from former `kowalski-mcp-transport`)
- **rmcp bootstrap** — [`serve`](src/serve.rs) at `/mcp` + `/health`
- **Output framing** — [`FrameKind`](src/framing.rs) / prompt-injection mitigation
- **Credential forwarding** — [`ForwardedHeaders`](src/headers.rs) for multi-tenant deployments

## Quick start (rmcp server)

```rust
use kowalski_mcp_base::{init_tracing, serve, ServeOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let bind = "127.0.0.1:8080".parse()?;
    serve(ServeOptions::new(bind), || MyServer::new()).await
}
```

## Quick start (`McpHandler` server)

See [`kowalski-mcp-rookery`](../kowalski-mcp-rookery/src/main.rs).

## Authoring rules

- [`MCP_REQUIREMENTS.md`](./MCP_REQUIREMENTS.md)
- [`MANIFEST_SPEC.md`](./MANIFEST_SPEC.md)
- [`AGENTS.md`](./AGENTS.md)

## Build

```bash
cargo test -p kowalski-mcp-base
```

MCP servers are optional workspace members — root `cargo build` does not compile them unless you pass `-p`.
