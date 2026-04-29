---
name = "knowledge-compiler-main"
available_agents = ["ingest", "compile", "ask", "lint"]
pipeline = ["ingest", "compile", "ask", "lint"]
default_question = "What changed in the latest source?"
---

# Knowledge Compiler Main Agent

Coordinates specialist sub-agents to ingest sources, compile wiki knowledge,
answer a question, and run integrity linting.
