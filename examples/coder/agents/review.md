---
name = "review"
kind = "process"
capability = "coder.process"
default_agent_id = "coder-review"
display_name = "Review"
description = "Code review notes and suggested adjustments."
prompt_file = "prompts/review.md"
output = "debug/stage-review.md"
context_paths = ["@artifact@"]
---

# Review
