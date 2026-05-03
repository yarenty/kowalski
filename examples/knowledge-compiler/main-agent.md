---
name = "knowledge-compiler-main"
available_agents = ["ingest", "compile", "ask", "lint", "research"]
pipeline = ["ingest", "compile", "ask", "lint"]
default_question = "What changed in the latest source?"
# Optional mdBook vault merge (paths relative to this folder):
# external_vault_root = "../dev_tips"
# mdbook_doc_rel = "doc"
# corpus_budget_chars = 120000
---

# Knowledge Compiler Main Agent

Coordinates specialist sub-agents to ingest sources, compile wiki knowledge,
answer a question, and run integrity linting.
