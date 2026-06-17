---
name = "todo-plan"
kind = "process"
capability = "coding-assistant.process"
default_agent_id = "coding-assistant-todo-plan"
display_name = "Todo plan"
description = "Break the user task into a prioritized todo list."
prompt_file = "prompts/todo-plan.md"
output = "debug/stage-todo-plan.md"
context_paths = ["@artifact@"]
---

# Todo plan
