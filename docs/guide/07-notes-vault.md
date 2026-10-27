# Notes vault (Obsidian)

Set **Notes vault** in Setup to your Obsidian vault's folder (or any folder of notes; it must
exist). Hordes that write notes, the Knowledge compiler and the Morning brief, then save each
finished note into the vault's **Kowalski** folder:

```text
Kowalski/2026-10-01 What are the main ways to learn Rust.md
```

Each note has front matter that Obsidian shows as properties: `source: kowalski`, the horde, the
run and the date, plus the note's own title and tags. A note is never overwritten; a second one
with the same name gets ` (2)`. The run's page says where the note went, with **Open in
Obsidian**. No plugin is needed: a vault is a folder, so Obsidian sees the file at once.
