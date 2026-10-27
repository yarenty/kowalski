# Build your own horde

A horde is a folder:

```text
my-horde/
├── horde.md        # name, description, the pipeline of steps, triggers
├── agents/*.md     # one file per step (a "penguin"): its kind, prompt, output
└── prompts/*.md    # what each model step is told
```

The easiest way to make one is **Build** (top bar): describe the chore in a conversation and the
Rookery writes the folder for you, validated. To change a built-in horde, copy its folder into
your hordes folder (`~/.config/kowalski/hordes/`) and edit it; your copy replaces the built-in
one.

Hordes move between machines as bundles: **Admin → Federation → Import bundle…** checks a
`.kwf.zip` first and lists anything it can't run on this machine. Imported hordes arrive with
their triggers switched off and run their steps in a separate process.

Step kinds include reading sources (`ingest`), model steps, `table_profile` / `sql_batch` /
`xlsx_report` for spreadsheets through tableski, and `verify` / `apply` for commands and project
changes (both ask first).
