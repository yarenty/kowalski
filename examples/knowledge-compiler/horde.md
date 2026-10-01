---
id = "knowledge-compiler"
display_name = "Knowledge Compiler"
description = "Turns the pages and notes you give it into one note that answers your question, with its sources, saved straight into your Obsidian vault."
capability_prefix = "kc"
category = "documents"
icon = "book"
featured = true
pipeline = ["ingest", "compile", "ask", "lint"]
default_question = "What changed in the latest source?"
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Your note"
vault = true
delivery_note = "When the run finishes, open **`workdir/PASTE_ME.md`** (final stage `output`). Intermediate files live under **`workdir/debug/`** per each agent’s declared `output` path."
delivery_root_rel = "PASTE_ME.md"
delivery_summary_note = "One note that answers your question from the pages you gave, with its sources. With a notes vault set it is already in your vault."
prompt_tip = "Ask a question, and list the pages or notes that should answer it."
---

# Knowledge Compiler

A small **markdown-staged app** (manifest + `agents/*.md`) for demo and federation tests.

## Sub-agents

- `ingest` (capability `kc.ingest`): captures sources under `workdir/debug/raw/` (see `agents/ingest.md`).
- `compile` (capability `kc.compile`): one digest file — path from `agents/compile.md` `output`.
- `ask` (capability `kc.ask`): question report — path from `agents/ask.md` `output`.
- `lint` (capability `kc.lint`): final handoff file — by default **`workdir/PASTE_ME.md`** (see `agents/lint.md`).

## Orchestration model

This app uses a simple **1:1 model**:

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
