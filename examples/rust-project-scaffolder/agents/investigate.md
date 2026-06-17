---
name = "investigate"
kind = "step"
capability = "rust-project-scaffolder.step"
default_agent_id = "rust-project-scaffolder-step"
display_name = "Crate and Dependency Investigation"
description = "Suggests relevant crates and dependencies based on the project goals."
prompt_file = "prompts/investigate.md"
output = "debug/stage-investigate.md"
context_paths = ["@artifact@"]
---

# Crate and Dependency Investigation

Suggests relevant crates and dependencies based on the project goals.
