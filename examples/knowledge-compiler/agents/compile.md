---
name = "compile"
kind = "compile"
capability = "kc.compile"
default_agent_id = "kc-compile"
display_name = "Compiler Agent"
description = "Turns ingest output into a structured markdown digest (single file)."
prompt_file = "prompts/compiler.md"
output = "debug/stage-compile.md"
context_paths = ["@artifact@"]
normalize_doc_title = "Source Summary"
normalize_sections = ["Summary", "Extracted Concepts", "Notable Claims", "Sources"]
normalize_fallback = "Fallback summary due to empty or malformed model output."
normalize_fallback_sections = ["Summary", "Extracted Concepts", "Notable Claims", "Sources"]
---

# Compiler Agent

Produces one markdown digest from the latest ingest artifact (`@artifact@` in context). No filesystem layout beyond the declared `output` path — the model follows `prompts/compiler.md`.
