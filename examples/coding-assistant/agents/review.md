---
name = "review"
kind = "process"
capability = "coding-assistant.process"
default_agent_id = "coding-assistant-review"
display_name = "Review"
description = "Code review notes and suggested adjustments."
prompt_file = "prompts/review.md"
output = "debug/stage-review.md"
context_paths = ["@artifact@"]
---

# Review
