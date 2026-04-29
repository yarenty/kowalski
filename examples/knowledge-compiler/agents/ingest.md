---
name = "ingest"
kind = "ingest"
capability = "kc.ingest"
default_agent_id = "kc-ingest"
display_name = "Ingest Agent"
description = "Collects raw source material and stores normalized markdown."
output = "raw/sources/"
---

# Ingest Agent

Collects raw source material and stores normalized markdown in `raw/sources/`.

When delegated `kc.ingest`, the worker fetches the URL (or captures input text), writes a timestamped source file, and returns its absolute path so the next stage can read it.
