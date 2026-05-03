---
name = "lint"
kind = "lint"
capability = "kc.lint"
default_agent_id = "kc-lint"
display_name = "Lint Agent"
description = "Checks consistency, coverage, and linkage quality of compiled wiki content."
prompt_file = "prompts/lint.md"
output = "debug/derived/lint/latest.md"
---

# Lint Agent

Checks consistency, coverage, and linkage quality of compiled wiki content.

When delegated `kc.lint`, the worker reads the wiki index, asks the LLM for an integrity report, writes the result under `debug/derived/lint/`, then writes **`PASTE_ME.md`** at the workdir root.
