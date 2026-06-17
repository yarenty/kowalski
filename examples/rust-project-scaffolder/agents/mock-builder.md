---
name = "mock-builder"
kind = "step"
capability = "rust-project-scaffolder.step"
default_agent_id = "rust-project-scaffolder-step"
display_name = "Initial MVP Mock Generation"
description = "Creates the first functional mock or Minimal Viable Product (MVP) code based on the defined structure and dependencies."
prompt_file = "prompts/mock-builder.md"
output = "debug/stage-mock-builder.md"
context_paths = ["@artifact@"]
---

# Initial MVP Mock Generation

Creates the first functional mock or Minimal Viable Product (MVP) code based on the defined structure and dependencies.
