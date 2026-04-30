---
id = "knowledge-compiler"
display_name = "Knowledge Sucking Swarm"
description = "A federated horde that ingests web sources, compiles Obsidian-ready knowledge, answers questions, and lints the resulting wiki."
capability_prefix = "kc"
pipeline = ["ingest", "compile", "ask", "lint"]
default_question = "What changed in the latest source?"
default_topic = "federation"
artifacts_root = "."
workdir = "/opt/ml/kowalski/examples/knowledge-compiler/output"
delivery_title = "Obsidian Delivery"
delivery_note = "Import the wiki folder into your Obsidian vault. Reports/lint are optional side artifacts."
delivery_root_rel = "wiki"
delivery_summary_note = "Knowledge Sucking Swarm ingests your source, compiles Obsidian-friendly notes, generates a focused answer, and validates note consistency."
prompt_tip = "Try: can you check https://yarenty.com and get summary into obsidian?"
---

# Knowledge Sucking Swarm

A multi-agent horde for transforming web sources into a maintained Obsidian-style wiki.

## Sub-agents

- `ingest` (capability `kc.ingest`): fetches and normalizes inputs into `workdir/raw/sources/`.
- `compile` (capability `kc.compile`): turns sources into a structured wiki summary and refreshes wiki concept stubs / index under `workdir/wiki/`.
- `ask` (capability `kc.ask`): answers the user question against `workdir/wiki/` context.
- `lint` (capability `kc.lint`): produces a quality report under `workdir/derived/lint/`.

## Orchestration model

This horde uses a simple **1:1 model**:

- each pipeline **step** has one dedicated **agent worker**
- each worker executes only its own step capability

The orchestrator runs steps sequentially:

```
ingest -> compile -> ask -> lint
```

Each delegation goes through `/api/federation/delegate`; the matching worker executes and publishes progress events through `/api/federation/publish`.

## Conversation event contract

Events published by the orchestrator and workers carry:

- `kind`: one of `run_started`, `task_assigned`, `task_started`, `agent_message`, `task_finished`, `run_finished`, `run_failed`.
- `run_id`: stable identifier for the run.
- `step`: pipeline phase label (`ingest` | `compile` | `ask` | `lint`).
- `from`: worker/agent id or `orchestrator`.
- `to`: optional addressee.
- `text`: short human-readable message.
- `artifact`: optional artifact path when relevant.
