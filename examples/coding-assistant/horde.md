---
id = "coding-assistant"
display_name = "Coding assistant (planning)"
description = "DAG horde: project warmup and todo planning in parallel, then adjustment and a fixed dev/test/review chain. Writes markdown plans only (no repo edits)."
capability_prefix = "coding-assistant"
pipeline = ["ingest", "warmup", "todo-plan", "adjust", "dev-1", "dev-2", "test-verify", "review", "check-complete", "summary", "deliver"]
default_question = "Execute the coding task on the given project."
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Coding assistant handoff"
delivery_note = "Markdown plan, proposed changes, and verification checklist for the operator (apply in your IDE or Cursor)."
delivery_root_rel = "HANDOFF.md"

[[edges]]
from = "ingest"
to = "warmup"

[[edges]]
from = "ingest"
to = "todo-plan"

[[edges]]
from = "warmup"
to = "adjust"

[[edges]]
from = "todo-plan"
to = "adjust"

[[edges]]
from = "adjust"
to = "dev-1"

[[edges]]
from = "dev-1"
to = "dev-2"

[[edges]]
from = "dev-2"
to = "test-verify"

[[edges]]
from = "test-verify"
to = "review"

[[edges]]
from = "review"
to = "check-complete"

[[edges]]
from = "check-complete"
to = "summary"

[[edges]]
from = "summary"
to = "deliver"
---

# Coding assistant (planning tier)

Fork after **ingest**: **warmup** (project context summary) and **todo-plan** (task breakdown) run in parallel, merge at **adjust**, then a fixed **dev → test → review → complete → summary → deliver** chain.

Validate: `cargo run -p kowalski-cli -- agent-app validate --path examples/coding-assistant`
