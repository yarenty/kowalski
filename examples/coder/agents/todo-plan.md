---
name = "todo-plan"
kind = "process"
capability = "coder.process"
default_agent_id = "coder-todo-plan"
display_name = "Todo plan"
description = "Break the user task into a prioritized todo list."
prompt_file = "prompts/todo-plan.md"
output = "debug/stage-todo-plan.md"
context_paths = ["@artifact@"]
---

# Todo plan
