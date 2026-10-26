# Schedules, watchers and approvals

## Triggers

A horde can start itself:

- **cron:** on a schedule, e.g. the Morning brief's `0 7 * * 1-5` (7:00 on weekdays);
- **watch:** when a file arrives in a folder, e.g. the Folder watcher's `inbox/`;
- **webhook:** when something posts to its route on kowalski's API.

On the horde's page, open **Schedules and watchers**: **Switch on** / **Switch off** (kept on the
server, the horde's files are untouched) and **Run now**. Built-in schedules and watchers ship
switched off: nothing runs, or costs, until you switch it on.

## Approvals

A step that would run a command (`verify`) or change files in a project (`apply`) stops and asks.
The run shows **Needs you** in the Runs list and a red strip on the Hordes home; open it and press
**Approve and continue**. One approval covers that step for the rest of the run.

For a step you trust, press **Always allow** instead: it approves this run and later runs of that
horde go through the step without stopping, as long as it does exactly the same thing (the same
command, the same folders). If the horde is edited to run something else, or a run points at
another project folder, it asks again. The run's activity log still notes each time it was let
through. **Setup → Always allowed** lists them; **Revoke** makes the step ask again.

## Runs

**Runs** lists every run across hordes, newest first, with filters for **Needs you** and
**Failed**. Each run keeps its result, the files it wrote, and its activity log.
