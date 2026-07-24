---
name = "generate-code"
kind = "process"
avatar = "process"
capability = "apps-builder-3000-horde.process"
default_agent_id = "apps-builder-3000-horde-process"
display_name = "Generate Code"
description = "Generates the basic Rust source file based on the provided application name."
prompt_file = "prompts/generate-code.md"
output = "debug/src/main.rs"
context_paths = ["@artifact@"]
---

# Generate Code

Generates the basic Rust source file based on the provided application name.
