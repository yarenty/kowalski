# kowalski-mcp-datafusion roadmap

Crate version **1.0.0**. Workspace: **[`../ROADMAP.md`](../ROADMAP.md)**.

## Near term
- [ ] Optional Parquet/multi-table registration via CLI flags (without breaking single-table smoke tests).
- [ ] Document performance expectations for large files (streaming vs load).

## Medium term
- [ ] Optional Ballista / distributed path (only if it fits the same MCP surface).

## Done (1.0.0)
- [x] Streamable HTTP MCP, `query_sql` / `get_schema` / `column_statistics`, Docker + smoke test.
