# Knowledge Compiler Example

This example shows how to run a local-first, markdown-native "knowledge compiler" workflow on top of Kowalski conventions.

## What this demonstrates
- Ingest raw sources into a stable folder contract.
- Compile source material into linked markdown in `wiki/`.
- Generate derived outputs (`reports`, `slides`, `notes`) in `derived/`.
- Run repeatable quality checks ("knowledge linting").

## Folder layout
```text
examples/knowledge-compiler/
├── config/
│   ├── agents.yaml
│   └── pipeline.yaml
├── prompts/
│   ├── compiler.md
│   ├── query.md
│   ├── lint.md
│   └── output.md
├── templates/
│   ├── concept.md
│   ├── source_summary.md
│   └── index.md
├── scripts/
│   ├── init.sh
│   ├── ingest.sh
│   ├── compile.sh
│   ├── ask.sh
│   └── lint.sh
├── raw/
├── wiki/
├── derived/
└── scratch/
```

## Quick start
From repo root:

```bash
bash examples/knowledge-compiler/scripts/init.sh
bash examples/knowledge-compiler/scripts/ingest.sh "https://example.com/article"
bash examples/knowledge-compiler/scripts/compile.sh
bash examples/knowledge-compiler/scripts/ask.sh "What are the core ideas?"
bash examples/knowledge-compiler/scripts/lint.sh
```

## How to integrate with Kowalski runtime
- Use `config/agents.yaml` roles to map one `TemplateAgent` per responsibility.
- Feed prompt files in `prompts/` as role/task instructions.
- Keep templates in `templates/` as hard output contracts for file-writing tools.
- Keep all generated assets in this example tree for deterministic runs.

## Notes
- Scripts are intentionally conservative and filesystem-first.
- This scaffold does not require Python; it uses shell and markdown contracts.
- Replace placeholder compile/query logic with your preferred Kowalski operator loop.
