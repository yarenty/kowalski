# Setup

**Setup** (top bar) writes `~/.config/kowalski/config.toml`. Saving restarts kowalski so every
agent picks up the change. You can also edit the file by hand.

## 1. Model

- **Local, free:** [Ollama](https://ollama.com) on your machine. Setup lists the models it has;
  pick one and press **Check the model**. No models yet: `ollama pull gemma4:e2b` (small, works on
  a CPU) or a larger one if your machine has the memory.
- **Hosted:** any OpenAI-compatible endpoint with your own key. The provider bills you per use.
  **Check the model** tests the key without spending tokens. The key is stored in the config file
  with owner-only permissions, or set `OPENAI_API_KEY` instead.

## 2. Files and the web

- **Folder:** Chat's file tool reads and writes only inside it. Setup creates it if it doesn't
  exist (the suggestion is `~/Documents/kowalski`).
- **Notes vault (optional):** an Obsidian vault, or any folder of notes. The folder must exist.
  See [Notes vault](07-notes-vault.md).
- **Web search:** DuckDuckGo works with no key but is unofficial and may refuse at times. For
  steadier search pick Brave, Staan, Tavily, Google via Serper or Google Programmable Search and
  paste its key (Google also needs a search engine ID). **Off** turns search off. Agents can
  always read a page you give them.

## 3. tableski (optional)

**Connect tableski** signs you in to tableski.io once (free plan, no card); kowalski keeps the
sign-in fresh and restarts so agents get its tools. See [tableski](06-tableski.md).

## Always allowed

Shown once you have pressed **Always allow** on an approval: each step that may run without
stopping, with what it does. **Revoke** makes it ask again. See
[Approvals](04-schedules-and-approvals.md#approvals).
