# Hire a horde in Paperclip

[Paperclip](https://github.com/paperclipai/paperclip) runs a company of AI agents: issues on a
board, agents that wake up on a heartbeat and work on what is assigned to them. A kowalski horde
can be one of those agents. Assign it an issue and the answers come back as a comment in the
issue's thread, worked out on your own machine by a small local model.

The bridge is one script, [`examples/paperclip/hire-a-horde.sh`](../../examples/paperclip/hire-a-horde.sh),
run by Paperclip's `process` adapter. On each heartbeat it:

1. takes the issue the agent was woken for (or else the oldest one assigned to it);
2. runs the horde with the issue's title and description as the request;
3. posts the answers as a comment and marks the issue **done**.

If the run fails, waits for an [approval](04-schedules-and-approvals.md#approvals), or kowalski is
not running, the issue is marked **blocked** with a comment saying what happened and what to do.

## Set it up

You need kowalski running on the same machine as Paperclip, and `curl` and `python3`.

The `process` adapter is not in Paperclip's hire form yet ("Coming soon"), so create the agent
through Paperclip's API. On a local Paperclip (trusted mode, port 3100):

```bash
curl -s -H 'content-type: application/json' http://localhost:3100/api/companies/<company-id>/agents -d '{
  "name": "Analyst (Kowalski)",
  "adapterType": "process",
  "adapterConfig": {
    "command": "/bin/bash",
    "args": ["/path/to/kowalski/examples/paperclip/hire-a-horde.sh", "spreadsheet-analyst", "questions"],
    "env": { "KOWALSKI_URL": "http://127.0.0.1:3456" },
    "timeoutSec": 600
  }
}'
```

- The first argument is the horde's id (shown in its page's address, `?horde=<id>`).
- The second is optional: for a horde with a form, the field the issue text goes in
  (`questions` for the Spreadsheet analyst). Without it the issue text is the horde's prompt.
- `KOWALSKI_URL` is where kowalski listens (`http://127.0.0.1:3456` by default); add
  `KOWALSKI_TOKEN` when kowalski runs with `--auth`.

Then assign the agent an issue, for example "Who spent the most across the orders?". Assigning
wakes it; the answer lands in the thread a minute or two later.

## Good to know

- One issue per heartbeat. Paperclip only lets an agent write to the issue its run was started
  for, so each assigned issue gets its own run.
- Steps that run a command stop for your approval in kowalski, and the issue shows **blocked**.
  Approve in kowalski, or use **Always allow** for a step you trust, then set the issue back to
  **todo**.
- Paperclip shows the answers in the run log too: the script prints them.
