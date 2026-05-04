---
name = "ask"
kind = "ask"
capability = "kc.ask"
default_agent_id = "kc-ask"
display_name = "Query Agent"
description = "Answers the operator question using prior stage markdown."
prompt_file = "prompts/query.md"
output = "debug/stage-ask-report.md"
context_paths = ["@artifact@"]
normalize_doc_title = "Answer"
normalize_sections = ["Question", "Response", "Sources Used"]
normalize_fallback = "Fallback answer due to empty or malformed model output."
normalize_fallback_sections = ["Question", "Response", "Sources Used"]
---

# Query Agent

Uses the chained previous artifact (`@artifact@` — the compile digest when the orchestrator runs `compile` before `ask`) plus `prompts/query.md` to write the report at `output`. For an all-local run, `@step:compile@` could be added in a copy of this app if you need both raw and digest without chaining.
