---
name = "warmup"
kind = "process"
capability = "coder.process"
default_agent_id = "coder-warmup"
display_name = "Project warmup"
description = "Summarize project context from intake (stack, layout, conventions)."
prompt_file = "prompts/warmup.md"
output = "debug/stage-warmup.md"
context_paths = ["@artifact@"]
---

# Project warmup
