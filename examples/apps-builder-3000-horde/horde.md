---
id = "apps-builder-3000-horde"
display_name = "Apps Builder 3000 Horde"
description = "A linear pipeline to ingest a project name and generate a simple Rust 'Hello World' application structure."
capability_prefix = "apps-builder-3000-horde"
pipeline = ["ingest", "generate-code", "deliver"]
default_question = "What should we do with the latest output?"
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Delivery"
delivery_note = "When the run finishes, open **`workdir/HANDOFF.md`**. Intermediates live under **`workdir/debug/`** per agent `output` paths."
delivery_root_rel = "HANDOFF.md"
delivery_summary_note = "A linear pipeline to ingest a project name and generate a simple Rust 'Hello World' application structure."
---

# Apps Builder 3000 Horde

A linear pipeline to ingest a project name and generate a simple Rust 'Hello World' application structure.

## Sub-agents (penguins)

- `ingest` (ingest): Gathers the required application name from the operator.
- `generate-code` (process): Generates the basic Rust source file based on the provided application name.
- `deliver` (deliver): Confirms the final artifact path and creates a Handoff summary.

## Orchestration model

Linear pipeline:

```
ingest -> generate-code -> deliver
```
