# Design: A2A at the federation edge (R4, target 1.4/1.5)

> **Status:** design-only (no code in 1.3.x). Companion to [`../PLAN.md`](../PLAN.md) §R4 and
> [`../ROADMAP.md`](../ROADMAP.md) ("Planned: A2A at the federation edge").
> **TL;DR:** Adopt [A2A](https://a2a-protocol.org/) (Agent2Agent) **only as the external skin at
> the node↔node boundary**, mapped onto the existing [`AclMessage`](../kowalski-core/src/federation/acl.rs)
> bus and [`AgentRegistry`](../kowalski-core/src/federation/registry.rs). **No penguin-to-penguin A2A.**

## 1. Decision and rationale

Kowalski already has a working internal **Agent Communication Language (ACL)**: `AclEnvelope` +
`AclMessage` over a broker (`MpscBroker` in-process, `PgBroker` via Postgres `NOTIFY`). Within a
horde, penguins do **not** talk peer-to-peer — the orchestrator (`kowalski/src/horde.rs`) advances a
**linear pipeline** and hands off via **artifact files** (`context_paths`, `@artifact@`). That model
is simple, debuggable, and sufficient (KISS, OCP). A2A's task/turn ceremony inside a horde would add
protocol overhead with no benefit.

The value of A2A is **interoperability between independent Kowalski nodes (and third-party agents)**:
a standard so an external agent can discover what a Kowalski node can do and delegate a task to it.
That is exactly the **federation boundary**. So:

| Layer | Protocol | Notes |
|-------|----------|-------|
| **Inside a horde** (penguin ↔ penguin) | none — orchestrator + artifact handoff | unchanged; no A2A |
| **Inside a node** (orchestrator ↔ workers, UI events) | ACL (`AclMessage`) over broker | unchanged; internal bus |
| **Node ↔ node / external agent** | **A2A** (Agent Card + Task lifecycle) | **new edge adapter**, translates to/from ACL |

A2A is an **adapter at the edge**, never a replacement for ACL. ACL stays the source of truth; the
A2A adapter is a translation shim (DIP: the orchestrator depends on ACL, not on A2A).

## 2. A2A primitives → Kowalski mapping

A2A has two core concepts: the **Agent Card** (discovery) and the **Task** (work lifecycle), exchanged
as **JSON-RPC 2.0 over HTTP**, with **SSE** for streaming updates.

### 2.1 Agent Card ↔ `AgentRecord` + node metadata

A2A publishes a JSON **Agent Card** (conventionally at `/.well-known/agent.json`) describing the
agent's identity, skills, endpoint URL, and auth. We already hold the substance in
[`AgentRecord`](../kowalski-core/src/federation/registry.rs) (`id`, `capabilities: Vec<String>`).

| A2A Agent Card field | Kowalski source |
|----------------------|-----------------|
| `name` / `description` | node config (operator-set) |
| `url` (service endpoint) | the node's HTTP base + an `/a2a` route |
| `skills[]` (id, name, description, tags) | `AgentRecord.capabilities` + horde catalog (`/api/hordes/*`) |
| `capabilities` (streaming, push) | `streaming: true` (we already do SSE); push later |
| `securitySchemes` / `authentication` | edge auth (see §4) |
| `version` / `protocolVersion` | crate version + pinned A2A version |

Generation is **derived, not hand-authored**: build the card from the registry + horde catalog so it
can't drift (same discipline as the MCP `tools/list` we just shipped).

### 2.2 A2A Task lifecycle ↔ `AclMessage`

A2A models work as a **Task** with a state machine and streamed updates/artifacts. Our `AclMessage`
horde-run variants already cover the same lifecycle — the adapter is a 1:1-ish translation:

| A2A task state / event | `AclMessage` (existing) |
|------------------------|--------------------------|
| `tasks/send` (create) + `submitted` | `TaskOffer` / `TaskDelegate` (depth-guarded), `RunStarted` |
| `working` (status update) | `TaskAssigned`, `TaskStarted`, `AgentMessage` |
| `input-required` | *(gap — see §5; maps to operator forms / Phase 8)* |
| `artifact` (TaskArtifactUpdateEvent) | `TaskFinished { artifact }`, `RunFinished { artifacts }` |
| `completed` | `RunFinished { handoff_markdown }` |
| `failed` / `canceled` | `RunFailed`, `Error` |
| streaming via SSE (`tasks/sendSubscribe`) | the federation SSE stream (`GET /api/federation/stream`) |

Delegation safety already exists: `check_delegate_depth` enforces `DEFAULT_MAX_DELEGATION_DEPTH`
(3) / `ABSOLUTE_MAX_DELEGATION_DEPTH` (32). The A2A adapter maps an inbound task to a depth-0
`TaskDelegate`/`RunStarted` and refuses cycles via the same guard.

## 3. Transport reuse

A2A is **JSON-RPC 2.0 over HTTP + SSE for streaming** — the *exact* shape of the
[`kowalski-mcp-transport`](../kowalski-mcp-transport/) stateless Streamable HTTP server we just built
(JSON or `text/event-stream`, notifications → `202`). The A2A edge endpoint should reuse that
transport (an `A2aHandler: McpHandler`-style dispatcher) rather than introduce a second HTTP stack.
This keeps one wire-handling code path and one stateless-HTTP discipline across MCP **and** A2A.

> Note: A2A and MCP are complementary. MCP = "agent calls **tools**"; A2A = "agent delegates to
> **another agent**". Kowalski is already an MCP client/server; A2A adds the peer-node skin.

## 4. Security (edge only)

- **Auth at the boundary:** Agent Card advertises a security scheme; inbound A2A calls carry a bearer
  token / mTLS terminated at the `/a2a` route. Internal ACL stays unauthenticated in-process.
- **Capability gating:** only skills derived from the registry are accepted; unknown `skill.id` →
  A2A error. Reuse the delegation-depth guard for loop/abuse protection.
- **No ambient trust:** an inbound A2A task is a *request*, validated like any operator input before
  it becomes a `RunStarted`.

## 5. Scope, non-goals, open questions

**In scope (1.4/1.5):** read-only Agent Card endpoint; inbound `tasks/send` → horde run; SSE task
updates from the federation stream; edge auth; depth-guarded delegation.

**Non-goals (now):** penguin-to-penguin A2A; replacing ACL; A2A push notifications; multi-tenant
auth; A2A "input-required" interactive turns (depends on Phase 8 runtime forms).

**Open questions:**
1. **`input-required`**: A2A supports mid-task input requests; we don't have runtime operator forms
   yet (Phase 8). Defer interactive A2A turns until Phase 8 lands, or stub as "fail with reason".
2. **Card hosting**: `/.well-known/agent.json` on the `kowalski` HTTP server vs a dedicated edge
   binary. Lean: serve from `kowalski` (one operator surface), derived from registry + catalog.
3. **Pin the A2A spec version** at implementation time and record it like `MCP_PROTOCOL_VERSION`.
4. **Outbound** (Kowalski as A2A *client* delegating to a remote agent): mirror the MCP client
   pattern; likely a follow-up after the inbound server.

## 6. Why this is "sound"

- **Standards on the wire, internals unchanged:** A2A (JSON-RPC/HTTP/SSE) is an open standard at the
  edge; ACL remains the internal bus. Same separation we use for MCP (see
  [`DESIGN_MEMORY_AND_DEPENDENCIES.md`](./DESIGN_MEMORY_AND_DEPENDENCIES.md) philosophy: minimize
  moving parts, keep optional things optional).
- **No new transport stack:** reuses `kowalski-mcp-transport`.
- **No orchestration rewrite:** the adapter translates to existing `AclMessage`; horde execution is
  untouched.
