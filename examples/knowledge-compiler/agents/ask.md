---
name = "ask"
kind = "ask"
capability = "kc.ask"
default_agent_id = "kc-ask"
display_name = "Query Agent"
description = "Answers a user question from compiled wiki context."
prompt_file = "prompts/query.md"
output = "derived/reports/latest.md"
---

# Query Agent

Answers a user question from compiled wiki context.

When delegated `kc.ask`, the worker reads the wiki index, formulates a grounded answer to the user question, and emits a markdown report under `derived/reports/`.
