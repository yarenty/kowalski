---
name = "warmup"
kind = "process"
capability = "coding-assistant.process"
default_agent_id = "coding-assistant-warmup"
display_name = "Project warmup"
description = "Summarize project context from intake (stack, layout, conventions)."
prompt_file = "prompts/warmup.md"
output = "debug/stage-warmup.md"
context_paths = ["@artifact@"]
---

# Project warmup
