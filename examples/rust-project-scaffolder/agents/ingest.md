---
name = "ingest"
kind = "ingest"
capability = "rust-project-scaffolder.ingest"
default_agent_id = "rust-project-scaffolder-ingest"
display_name = "Project Input Reception"
description = "Receives and parses the initial requirements and goals for the Rust project."
prompt_file = "prompts/ingest.md"
output = "debug/raw/"
[[inputs]]
id = "project_name"
type = "text"
label = "Project name"
required = true
placeholder = "my-rust-service"
[[inputs]]
id = "project_goals"
type = "textarea"
label = "Goals and constraints"
required = true
placeholder = "e.g. CLI tool, async HTTP, SQLite, no cloud deps…"
[[inputs]]
id = "repo_url"
type = "url"
label = "Existing repository URL (optional)"
placeholder = "https://github.com/org/repo"
[[inputs]]
id = "crate_focus"
type = "choice"
label = "Primary project shape"
options = ["cli", "web-api", "library", "embedded"]
default = "cli"
---

# Project Input Reception

Receives and parses the initial requirements and goals for the Rust project.
