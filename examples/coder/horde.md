---
id = "coder"
display_name = "Coder"
description = "DAG coding horde: project ingest, parallel planning, tool-enabled dev stages, verify with retry loop, review and handoff."
capability_prefix = "coder"
pipeline = ["ingest", "warmup", "todo-plan", "adjust", "dev-1", "dev-2", "test-verify", "review", "check-complete", "summary", "deliver"]
default_question = "Execute the coding task on the given project."
default_topic = "federation"
artifacts_root = "."
workdir = "output"
delivery_title = "Coder handoff"
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
when = "pass"

[[edges]]
from = "test-verify"
to = "dev-1"
when = "fail"
max_loops = 2

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

# Coder (planning tier)

Fork after **ingest**: **warmup** (project context summary) and **todo-plan** (task breakdown) run in parallel, merge at **adjust**, then a fixed **dev → test → review → complete → summary → deliver** chain.

Validate: `cargo run -p kowalski-cli -- agent-app validate --path examples/coder`
