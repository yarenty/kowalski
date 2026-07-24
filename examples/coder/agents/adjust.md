---
name = "adjust"
kind = "process"
capability = "coder.process"
default_agent_id = "coder-adjust"
display_name = "Adjust & merge"
description = "Reconcile warmup summary with todo plan into one execution brief."
prompt_file = "prompts/adjust.md"
output = "debug/stage-adjust.md"
context_paths = ["@step:warmup@", "@step:todo-plan@"]
---

# Adjust & merge
