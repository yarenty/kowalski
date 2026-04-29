---
id = "knowledge-compiler"
display_name = "Knowledge Sucking Swarm"
description = "A federated horde that ingests web sources, compiles Obsidian-ready knowledge, answers questions, and lints the resulting wiki."
capability_prefix = "kc"
pipeline = ["ingest", "compile", "ask", "lint"]
default_question = "What changed in the latest source?"
default_topic = "federation"
artifacts_root = "."
delivery_title = "Obsidian Delivery"
delivery_note = "Import the wiki folder into your Obsidian vault. Reports/lint are optional side artifacts."
delivery_root_rel = "wiki"
delivery_summary_note = "Knowledge Sucking Swarm ingests your source, compiles Obsidian-friendly notes, generates a focused answer, and validates note consistency."
---

# Knowledge Sucking Swarm

A multi-agent horde for transforming web sources into a maintained Obsidian-style wiki.

## Sub-agents

- `ingest` (capability `kc.ingest`): fetches and normalizes a source URL into `raw/sources/`.
- `compile` (capability `kc.compile`): turns the raw source into a structured wiki summary and refreshes wiki concept stubs / index.
- `ask` (capability `kc.ask`): answers the user question against the wiki context.
- `lint` (capability `kc.lint`): produces a quality report about wiki consistency and link health.

## Orchestration model

This horde uses **one-worker-per-sub-agent** federation. The orchestrator delegates sequentially:

```
ingest -> compile -> ask -> lint
```

Each delegation goes through `/api/federation/delegate`, the matching worker executes, and publishes structured inter-agent conversation events back through `/api/federation/publish`.

## Conversation event contract

Events published by orchestrator and workers (via federation broker) carry:

- `kind`: one of `run_started`, `task_assigned`, `task_started`, `agent_message`, `task_finished`, `run_finished`, `run_failed`.
- `run_id`: stable identifier for the run.
- `step`: `ingest` | `compile` | `ask` | `lint`.
- `from`: agent id or `orchestrator`.
- `to`: optional addressee.
- `text`: short human-readable message.
- `artifact`: optional artifact path string when relevant.
