---
id = "folder-watcher"
display_name = "Folder Watcher"
description = "Drop a document into the inbox folder and get a short note: what it is, the key facts (amounts, dates, people), and what you need to do by when. Switch the watcher on in the Horde tab."
capability_prefix = "folder-watcher"
pipeline = ["ingest", "note"]
default_question = "Summarise the new document."
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Your note"
delivery_note = "Open **`workdir/NOTE.md`** for the latest document. Every document read is kept in `debug/raw/`, one file per run."
delivery_root_rel = "NOTE.md"
delivery_summary_note = "A short note on the document that just arrived."
prompt_tip = "Paste a file path (or several), or switch on the watcher and drop files into the inbox folder."

[[triggers]]
watch = { path = "inbox", events = ["create"], debounce_ms = 2000 }
enabled = false
---

# Folder Watcher

Watches the `inbox/` folder next to this file. Each new text document (`.txt`, `.md`, `.csv`,
`.html`, exported emails) becomes `NOTE.md`: what it is, the facts that matter, and the actions
with their deadlines.

## Steps (penguins)

- `ingest` (ingest): reads the new file(s) into `debug/raw/`.
- `note` (process): writes the note from the document, and nothing else.

## Switching it on

The watcher ships switched off: turn it on in the Horde tab, then drop files into `inbox/`. A run
can also be started by hand with one or more file paths.
