---
name = "structure"
kind = "step"
capability = "rust-project-scaffolder.step"
default_agent_id = "rust-project-scaffolder-step"
display_name = "Repository Structure Creation"
description = "Creates the initial directory and file structure for the new Rust repository."
prompt_file = "prompts/structure.md"
output = "debug/stage-structure.md"
context_paths = ["@artifact@"]
---

# Repository Structure Creation

Creates the initial directory and file structure for the new Rust repository.
