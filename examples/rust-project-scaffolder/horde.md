---
id = "rust-project-scaffolder"
display_name = "Rust Project Scaffolder Pipeline"
description = "A pipeline designed to automate the setup of a new Rust project, including repository structure, dependency investigation, MVP creation, and task planning."
capability_prefix = "rust-project-scaffolder"
pipeline = ["ingest", "structure", "investigate", "mock-builder", "todo-generator", "deliver"]
default_question = "What should we do with the latest output?"
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Delivery"
delivery_note = "When the run finishes, open **`workdir/Final structured report`**. Intermediates live under **`workdir/debug/`** per agent `output` paths."
delivery_root_rel = "HANDOFF.md"
delivery_summary_note = "A pipeline designed to automate the setup of a new Rust project, including repository structure, dependency investigation, MVP creation, and task planning."
---

# Rust Project Scaffolder Pipeline

A pipeline designed to automate the setup of a new Rust project, including repository structure, dependency investigation, MVP creation, and task planning.

## Sub-agents (penguins)

- `ingest` (step): Receives and parses the initial requirements and goals for the Rust project.
- `structure` (step): Creates the initial directory and file structure for the new Rust repository.
- `investigate` (step): Suggests relevant crates and dependencies based on the project goals.
- `mock-builder` (step): Creates the first functional mock or Minimal Viable Product (MVP) code based on the defined structure and dependencies.
- `todo-generator` (step): Synthesizes all previous steps into a concrete to-do list for the next development steps.
- `deliver` (final): Compiles and delivers the final structured output.

## Orchestration model

Linear pipeline (1.3.0):

```
ingest -> structure -> investigate -> mock-builder -> todo-generator -> deliver
```
