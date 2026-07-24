---
name = "test-verify"
kind = "verify"
capability = "coder.verify"
default_agent_id = "coder-test-verify"
display_name = "Test verification"
description = "Runs configured verification command in the operator project (e.g. cargo test)."
verify_command = "cargo test --workspace --quiet"
output = "debug/stage-test-verify.md"
context_paths = ["@artifact@"]
---

# Test verification

Runs `verify_command` in the operator project directory and writes stdout/stderr to the stage artifact.
