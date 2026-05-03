---
name = "research"
kind = "research"
capability = "kc.research"
default_agent_id = "kc-research"
display_name = "Research Agent"
description = "Expands a raw seed (ingest output) into a structured investigation packet."
prompt_file = "prompts/research_seed.md"
output = "derived/research/latest.md"
---

# Research Agent

Runs after **ingest** (or with the same source string) to produce a structured **Investigation Packet** for a short tip, product name, or URL list.  
Add `research` to the `pipeline` in `main-agent.md` when you want this step; start a worker with `--role research` if using federation.
