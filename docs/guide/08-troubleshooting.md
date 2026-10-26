# Troubleshooting

## "Ollama is not running" / no models in Setup

Start Ollama (`ollama serve`, or open the
Ollama app), then `ollama pull gemma4:e2b`, and press **Check the model** again.

## A run is very slow

A local model on a CPU takes minutes per step; that's normal. A smaller
model (`gemma4:e2b`) or a hosted model with your key is faster.

## "tool … is not available" (Spreadsheet analyst)

tableski isn't connected or isn't running.
Connect it in Setup, or start your local tableski; kowalski keeps retrying and picks it up.

## The Workbooks card says the tableski runs locally

A local tableski has no upload: start it
with your file (`tableski --file your.xlsx`).

## A run says Needs you

A step wants to run a command or change files. Open the run and press
**Approve and continue**, or cancel it. **Always allow** stops it asking for that step of that
horde; revoke it in **Setup → Always allowed**.

## A run failed

Open it from **Runs**: the failed step and the reason are at the top. Common
causes: the model isn't available, a page couldn't be fetched (the run notes which), or a step
timed out.

## Web search fails

DuckDuckGo limits automated searches. Add a Brave, Staan or Tavily key in
Setup.

## Save in Setup fails

The message is next to the Save button. A notes vault must be an
existing folder; the files folder is created for you.

## Where are my files?

Everything is beside `~/.config/kowalski/config.toml`: the built-in hordes
(`builtin-hordes/`), your hordes (`hordes/`) and the database (`db/`). The Mac app logs to
`~/Library/Logs/Kowalski/kowalski.log`.
