---
name = "compile"
kind = "compile"
capability = "kc.compile"
default_agent_id = "kc-compile"
display_name = "Compiler Agent"
description = "Compiles raw source into structured wiki markdown."
prompt_file = "prompts/compiler.md"
output = "wiki/summaries/latest.md"
---

# Compiler Agent

Compiles raw source into structured wiki markdown and updates summary artifacts.

When delegated `kc.compile`, the worker reads the latest raw source provided in the instruction, asks the LLM to summarize and extract concepts, normalizes the resulting markdown, repairs concept backlinks, and rebuilds the wiki index.
