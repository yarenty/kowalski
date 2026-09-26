---
name = "brief"
kind = "process"
avatar = "process"
capability = "morning-brief.brief"
default_agent_id = "morning-brief-brief"
display_name = "Write the brief"
description = "One page: the items worth your time, why they matter, and links."
prompt_file = "prompts/brief.md"
output = "BRIEF.md"
context_max_chars = 14000
context_paths = ["@artifact@"]
---

# Write the brief

Turns the fetched pages into a one-page brief.
