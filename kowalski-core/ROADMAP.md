# kowalski-core roadmap

Crate version **1.0.0** (see `Cargo.toml`). For the whole workspace, see **[`../ROADMAP.md`](../ROADMAP.md)**.

## Near term
- [ ] Broader integration tests (mock LLM / contract tests for tool JSON edge cases).
- [ ] Optional: additional `LLMProvider` backends as thin adapters only when needed.

## Medium term
- [ ] Memory: conversation search / indexing (if kept in-tree; align with [`docs/DESIGN_MEMORY_AND_DEPENDENCIES.md`](../docs/DESIGN_MEMORY_AND_DEPENDENCIES.md)).
- [ ] Federation: stricter ACL defaults and operator docs for Postgres `LISTEN/NOTIFY`.

## Done (1.0.0 baseline)
- [x] `BaseAgent` / `TemplateAgent`, `chat_with_tools`, streaming helpers.
- [x] MCP client (Streamable HTTP / SSE), hub + tool proxies.
- [x] Optional Postgres + pgvector path; graph status + AGE Cypher helpers behind `postgres` feature.
