# Architecture v03 (future)

Future-facing architecture sketch for roadmap planning.

- Diagram source: [`img/architecture_v03-future.excalidraw`](./img/architecture_v03-future.excalidraw)
- Current status diagram: [`architecture_v02.md`](./architecture_v02.md)

```mermaid
flowchart TB
  subgraph Interfaces
    UI[Vue UI]
    CLI[CLI + agent-app]
    API[Public API]
    SDK[SDK / facade crate]
  end

  subgraph Orchestration
    Orchestrator[Agent orchestration service]
    Registry[Federation registry]
    Policy[Policy + RBAC + guardrails]
    Obs[Telemetry + tracing + audit]
  end

  subgraph Execution
    Runtime[TemplateAgent runtime]
    ToolBus[Tool bus + MCP adapters]
    Workflows[Reusable workflow engine]
  end

  subgraph Data
    Episodic[(SQL episodic store)]
    Semantic[(Vector/semantic store)]
    Graph[(Optional graph model)]
    State[(Agent state + tasks)]
  end

  subgraph Integrations
    MCP[MCP ecosystem]
    Queues[Queue/event backbone]
    Clouds[Managed deployment targets]
  end

  UI --> API
  CLI --> API
  SDK --> API
  API --> Orchestrator
  Orchestrator --> Runtime
  Orchestrator --> Registry
  Orchestrator --> Policy
  Orchestrator --> Obs
  Runtime --> ToolBus
  Runtime --> Workflows
  Runtime --> Episodic
  Runtime --> Semantic
  Runtime --> Graph
  Runtime --> State
  ToolBus --> MCP
  Orchestrator --> Queues
  Orchestrator --> Clouds
```
