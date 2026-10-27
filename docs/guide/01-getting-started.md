# Getting started

Kowalski is a small team of AI agents that runs on your own computer. You give it a chore; a
**horde**, a short pipeline of agents called penguins, does it and hands you the result: a report
workbook, a one-page brief, a note.

## Install

- **Mac:** download [Kowalski.dmg](https://github.com/yarenty/kowalski/releases/latest/download/Kowalski.dmg),
  open it, drag **Kowalski** to Applications and open it. It starts kowalski and opens your
  browser. Quit it from the Dock to stop kowalski. Its log is in `~/Library/Logs/Kowalski/kowalski.log`.
- **Mac or Linux, from a terminal:**
  `curl -fsSL https://raw.githubusercontent.com/yarenty/kowalski/main/install.sh | bash`, then
  `kowalski`. The installer puts `kowalski` and `kowalski-cli` in `~/.local/bin` and adds that
  folder to your PATH for new terminals.

## First start

The browser opens on **Setup**: a model, a files folder, and optionally tableski, a notes vault
and web search (see [Setup](02-setup.md)). Save, and kowalski restarts with your settings.
Everything lives beside the config file, `~/.config/kowalski/config.toml`.

## First run

Open **Hordes**, pick a tile (or press ⌘K / Ctrl+K and type its name), fill in its form, and press
**Send in the horde**. The page shows each step as it runs, then the result. Every run is listed
on **Runs**, named after what you asked.

A local model on a laptop CPU is slow but free: a one-page summary takes about two minutes on
`gemma4:e2b`. A hosted model with your own key is much faster.
