//! Minimal linear horde draft for tests and smoke runs.

use crate::rookery::types::{PenguinSpec, RookeryDraft};

/// Three-step linear horde: ingest → process → deliver (generic LLM + ingest).
pub fn minimal_linear_draft() -> RookeryDraft {
    RookeryDraft {
        id: "rookery-demo".into(),
        display_name: "Rookery Demo Horde".into(),
        description: "Minimal born horde for validate and agent-app smoke.".into(),
        capability_prefix: None,
        pipeline: vec![
            "collect".into(),
            "process".into(),
            "deliver".into(),
        ],
        edges: vec![],
        penguins: vec![
            PenguinSpec {
                name: "collect".into(),
                kind: "ingest".into(),
                display_name: "Collect Agent".into(),
                description: "Captures source input into the workdir.".into(),
                prompt_body: "Ingest stage: source is provided by the operator run input.".into(),
                agent_body: None,
                output: "debug/raw/".into(),
                context_paths: vec![],
                tool_ids: vec![],
                model_id: None,
                inputs: vec![],
                avatar: None,
            },
            PenguinSpec {
                name: "process".into(),
                kind: "process".into(),
                display_name: "Process Agent".into(),
                description: "Transforms the collected artifact.".into(),
                prompt_body: "You are a **process** stage.\n\nSummarize and structure the attached source. Use clear markdown sections.\n".into(),
                agent_body: None,
                output: "debug/stage-process.md".into(),
                context_paths: vec!["@artifact@".into()],
                tool_ids: vec![],
                model_id: None,
                inputs: vec![],
                avatar: None,
            },
            PenguinSpec {
                name: "deliver".into(),
                kind: "deliver".into(),
                display_name: "Deliver Agent".into(),
                description: "Produces the final handoff artifact.".into(),
                prompt_body: "You are a **deliver** stage.\n\nProduce a concise final handoff document from the context.\n".into(),
                agent_body: None,
                output: "HANDOFF.md".into(),
                context_paths: vec!["@artifact@".into()],
                tool_ids: vec![],
                model_id: None,
                inputs: vec![],
                avatar: None,
            },
        ],
        default_question: Some("What is the key takeaway?".into()),
        default_topic: Some("federation".into()),
        workdir: Some("output".into()),
        delivery_title: Some("Handoff".into()),
        delivery_note: None,
        delivery_root_rel: Some("HANDOFF.md".into()),
        delivery_summary_note: None,
        prompt_tip: None,
    }
}
