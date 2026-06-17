---
name = "test-verify"
kind = "process"
capability = "coding-assistant.process"
default_agent_id = "coding-assistant-test-verify"
display_name = "Test verification"
description = "Test plan and commands to verify the proposed changes."
prompt_file = "prompts/test-verify.md"
output = "debug/stage-test-verify.md"
context_paths = ["@artifact@"]
---

# Test verification
