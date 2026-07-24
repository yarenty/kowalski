# MCP requirements (first-party Kowalski servers)

> Normative rules for **first-party** MCP servers in this repository (`kowalski-mcp-*`).


## 1. Summary (the rules)

Every new first-party MCP server **MUST**:

1. **Frame tool output** so the model treats it as *data*, never *instructions* — use [`FrameKind`](src/framing.rs) from `kowalski-mcp-base`).
2. Be **stateless per request** — no per-user server state; each tool call stands alone.
3. Be **async Rust** — `tokio`, non-blocking, safe under concurrent calls.
4. Use **streamable HTTP** as the primary deployment transport; stdio for desktop clients.
5. Be built on **`kowalski-mcp-base`** — do not reimplement transport, health, or framing.
6. Have **no dev/test auth bypasses** — same code path in dev, CI, and production.
7. Ship a **`manifest.yaml`** beside the crate.

## 2. Content-aware output framing (REQUIRED)

Wrap tool text in BEGIN/END delimiters with a per-call nonce. Pick the correct [`FrameKind`](src/framing.rs):

| Source | `FrameKind` | When |
|--------|-------------|------|
| Web / external pages | `ExternalWeb` | Search, fetch-url |
| Vault / wiki / notes | `TrustedReference` | Obsidian, Knowledge Compiler outputs |
| SQL / stats / computed | `Computed` | DataFusion, calculators |

Use [`frame`](src/framing.rs) or [`structured_framed`](src/framing.rs) at the tool boundary. Include a **framing breakout test** in unit tests.

## 3. Stateless (REQUIRED)

- No session-scoped user data between tool calls.
- Kowalski default: single operator; credentials come from env/config at process start or per-request headers when deployed multi-tenant.
- Never cache one user's upstream credentials for another user's call.

## 4. Async Rust (REQUIRED)

- Edition 2024, `tokio`, `reqwest` with async I/O where applicable.
- CPU-bound work → `spawn_blocking`.
- Upstream calls have explicit timeouts; return normalized errors, never panic on upstream failure.

## 5. Transport (REQUIRED)

Two supported patterns from `kowalski-mcp-base`:

| Style | API | Use when |
|-------|-----|----------|
| Hand-rolled JSON-RPC | [`McpHandler`](src/transport.rs) + [`run_stdio`](src/transport.rs) / [`serve_http`](src/transport.rs) | Existing servers (datafusion, rookery) |
| rmcp `ServerHandler` | [`serve`](src/serve.rs) at `/mcp` + `/health` | New `#[tool_router]` servers (Obsidian, …) |

**Stateless HTTP:** no `Mcp-Session-Id` required. Every POST is independent.

**stdio:** logs to **stderr** only; stdout is the protocol stream.

## 6. Build on `kowalski-mcp-base` (REQUIRED)

```rust
use kowalski_mcp_base::{init_tracing, serve, ServeOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    kowalski_mcp_base::init_tracing();
    let bind = "0.0.0.0:8000".parse()?;
    kowalski_mcp_base::serve(ServeOptions::new(bind), || MyServer::new()).await
}
```

Or for `McpHandler` servers, see [`kowalski-mcp-rookery`](../kowalski-mcp-rookery/src/main.rs).

## 7. No auth shortcuts (REQUIRED)

Never add `DEV_MODE`, hardcoded tokens, or "skip auth in test" branches. Fix the real path instead.

## 8. Manifest (REQUIRED)

Each server crate includes **`manifest.yaml`** per [`MANIFEST_SPEC.md`](./MANIFEST_SPEC.md). Kowalski registers servers in `resources/config.toml`; manifests document tools, defaults, and smoke `test_tool` for operators and future catalog import.

## 9. Optional workspace builds

MCP server crates are **workspace members** but **not** in `default-members`. Build explicitly:

```bash
cargo build -p kowalski-mcp-base          # framework only
cargo build -p kowalski-mcp-datafusion
cargo build -p kowalski-mcp-rookery
cargo test -p kowalski-mcp-base -p kowalski-mcp-rookery
```

Root `cargo build` compiles only `kowalski`, `kowalski-core`, `kowalski-cli`.

## 10. Checklist for a new server

- [ ] Crate `kowalski-mcp-<name>` with thin tool logic
- [ ] Depends on `kowalski-mcp-base` only for transport/framing (not duplicated axum/rmcp wiring)
- [ ] Output framing + breakout test
- [ ] Stateless HTTP + optional stdio
- [ ] `manifest.yaml` + README + AGENTS.md
- [ ] Example `[[mcp.servers]]` block in `resources/config.toml`
- [ ] Root `CHANGELOG.md` when user-visible
