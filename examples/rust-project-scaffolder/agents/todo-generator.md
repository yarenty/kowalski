---
name = "todo-generator"
kind = "step"
capability = "rust-project-scaffolder.step"
default_agent_id = "rust-project-scaffolder-step"
display_name = "Task List Generation"
description = "Synthesizes all previous steps into a concrete to-do list for the next development steps."
prompt_file = "prompts/todo-generator.md"
output = "debug/stage-todo-generator.md"
context_paths = ["@artifact@"]
---

# Task List Generation

Synthesizes all previous steps into a concrete to-do list for the next development steps.
