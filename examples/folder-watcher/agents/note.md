---
name = "note"
kind = "process"
avatar = "process"
capability = "folder-watcher.note"
default_agent_id = "folder-watcher-note"
display_name = "Write the note"
description = "What it is, the key facts, and what to do by when."
prompt_file = "prompts/note.md"
output = "NOTE.md"
context_max_chars = 16000
context_paths = ["@artifact@"]
---

# Write the note

Turns the new document into a short, actionable note.
