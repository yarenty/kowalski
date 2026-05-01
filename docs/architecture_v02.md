# Architecture v02 (current)

Current 1.1.x architecture focused on the delivered workspace status.

- Diagram source: [`img/architecture_v02.excalidraw`](./img/architecture_v02.excalidraw)
- Companion future diagram: [`architecture_v03_future.md`](./architecture_v03_future.md)

```mermaid
flowchart TB
  subgraph Clients
    CLI[kowalski-cli]
    UI[ui (Vue)]
    APIUsers[HTTP integrations]
  end

  subgraph Runtime
    HTTP[kowalski binary /api/*]
    Core[kowalski-core TemplateAgent]
    Ext[extension + agent-app]
  end

  subgraph CoreModules[kowalski-core modules]
    Tools[Built-in tools + MCP tool proxy]
    Memory[Working + Episodic + Semantic memory]
    Fed[Federation types and flows]
  end

  subgraph Optional
    DF[kowalski-mcp-datafusion]
    PG[(Postgres + pgvector + AGE)]
  end

  CLI --> Ext --> HTTP
  UI --> HTTP
  APIUsers --> HTTP
  HTTP --> Core
  Core --> Tools
  Core --> Memory
  Core --> Fed
  Core <-->|MCP| DF
  Memory <-->|optional| PG
  Fed <-->|optional| PG
```
