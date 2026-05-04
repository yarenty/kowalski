---
name = "lint"
kind = "lint"
capability = "kc.lint"
default_agent_id = "kc-lint"
display_name = "Handoff Agent"
description = "Final handoff: YAML tags only; thematic content as ## headings + bodies in markdown (prompts/lint.md)."
prompt_file = "prompts/lint.md"
output = "PASTE_ME.md"
context_paths = [
  "debug/stage-compile.md",
  "debug/stage-ask-report.md",
]
normalize_doc_title = "Vault paste pack"
normalize_sections = [
  "TL;DR",
  "Themed extracts",
  "Answer recap",
  "Consistency and gaps",
  "Sources and follow-ups",
]
normalize_fallback = "- Model output was empty; re-run when the LLM endpoint is reachable.\n"
normalize_fallback_sections = ["TL;DR", "Answer recap"]
---

# Handoff agent (example)

Reads the fixed relative paths under the workdir that match earlier stages’ `output` fields, then emits **`PASTE_ME.md`** at the workdir root per `prompts/lint.md`. Rename this agent or paths in another app — nothing in Rust assumes this layout.
