//! **Rookery** — horde builder: validate drafts and write markdown-native horde trees.
//!
//! 1.3.0 supports **linear** pipelines (`horde.md` `pipeline = [...]`). Optional **`edges[]`**
//! (DAG scheduling) is validated via [`crate::horde_graph`] (1.5.0+).

mod avatars;
mod delta;
mod draft_parse;
mod fixture;
mod interview;
mod normalize;
mod repair;
mod types;
mod validate;
mod writer;

pub use avatars::{assign_penguin_avatars, infer_penguin_avatar};
pub use delta::{
    apply_batch, apply_op, base_delta_schema, build_delta_schema, ensure_schema_supported,
    unsupported_schema_feature, BatchError, BatchOutcome, DeltaBatch, DeltaOp, DeltaSchemaOptions,
    DELTA_SCHEMA_JSON,
};
pub use draft_parse::{extract_json_block, parse_draft_from_assistant};
pub use fixture::{minimal_dag_draft, minimal_linear_draft};
pub use interview::{
    ops_call_messages, run_ops_phase, turn_note, InterviewConfig, OpsModel, OpsPhase,
    OPS_SYSTEM_PROMPT,
};
pub use normalize::{
    default_output_for_penguin, normalize_draft, normalize_penguin_output, output_looks_invalid,
    slugify_horde_id,
};
pub use repair::repair_horde_tree_outputs;
pub use types::{HordeBirthSpec, PenguinSpec, RookeryDraft};
pub use validate::{
    validate_draft, validate_draft_with, validate_horde_id, validate_horde_tree,
    validate_horde_tree_report,
    validate_step_name, validate_workdir_relative_path, DraftStrictness,
};
pub use writer::{horde_root_path, write_horde_tree};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown_pipeline::{parse_app_manifest, parse_stage_agent, resolve_manifest_path};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn minimal_draft_validates() {
        let draft = minimal_linear_draft();
        validate_draft(&draft).expect("fixture draft should validate");
    }

    #[test]
    fn write_fixture_round_trip() {
        let dir = tempdir().unwrap();
        let spec = HordeBirthSpec::new(minimal_linear_draft());
        let root = write_horde_tree(dir.path(), &spec).unwrap();
        validate_horde_tree(&root).expect("written tree should validate");

        let manifest = parse_app_manifest(&resolve_manifest_path(&root)).unwrap();
        assert_eq!(manifest.id, "rookery-demo");
        assert_eq!(manifest.pipeline.len(), 3);

        let agent_path = root.join("agents/process.md");
        let stage = parse_stage_agent(&agent_path).unwrap();
        assert_eq!(stage.name, "process");
        assert_eq!(stage.kind, "process");
        assert!(stage.prompt_file.as_deref().is_some_and(|p| p.contains("process")));

        assert!(root.join("prompts/collect.md").is_file());
        assert!(root.join("README.md").is_file());
        assert!(root.join("AGENTS.md").is_file());
    }

    #[test]
    fn write_refuses_existing_without_overwrite() {
        let dir = tempdir().unwrap();
        let spec = HordeBirthSpec::new(minimal_linear_draft());
        write_horde_tree(dir.path(), &spec).unwrap();
        let err = write_horde_tree(dir.path(), &spec).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn write_overwrite_replaces_tree() {
        let dir = tempdir().unwrap();
        let mut draft = minimal_linear_draft();
        draft.description = "v1".into();
        write_horde_tree(dir.path(), &HordeBirthSpec::new(draft)).unwrap();

        let mut draft2 = minimal_linear_draft();
        draft2.description = "v2".into();
        let root =
            write_horde_tree(dir.path(), &HordeBirthSpec::new(draft2).with_overwrite(true)).unwrap();
        let body = fs::read_to_string(root.join("horde.md")).unwrap();
        assert!(body.contains("v2"));
    }

    /// A draft with all three trigger kinds writes `[[triggers]]` blocks that re-parse into
    /// the exact same triggers (writer emits every field, defaults included).
    #[test]
    fn write_triggers_round_trip() {
        use crate::horde_trigger::{HordeTrigger, WatchTrigger, WebhookTrigger};
        let dir = tempdir().unwrap();
        let mut draft = minimal_linear_draft();
        draft.triggers = vec![
            HordeTrigger {
                cron: Some("0 7 * * *".into()),
                watch: None,
                webhook: None,
                enabled: true,
                overlap: "skip".into(),
                input: std::collections::BTreeMap::new(),
                prompt: Some("Daily digest at {{trigger.time}}\nSecond line.".into()),
            },
            HordeTrigger {
                cron: None,
                watch: Some(WatchTrigger {
                    path: "inbox".into(),
                    events: vec!["create".into(), "modify".into()],
                    debounce_ms: 500,
                }),
                webhook: None,
                enabled: false,
                overlap: "queue".into(),
                input: std::collections::BTreeMap::new(),
                prompt: None,
            },
            HordeTrigger {
                cron: None,
                watch: None,
                webhook: Some(WebhookTrigger {
                    route: "demo-ingest".into(),
                }),
                enabled: true,
                overlap: "skip".into(),
                input: std::collections::BTreeMap::from([(
                    "question".to_string(),
                    "digest".to_string(),
                )]),
                prompt: Some("Payload: {{trigger.payload}}".into()),
            },
        ];
        let expected = draft.triggers.clone();
        let root = write_horde_tree(dir.path(), &HordeBirthSpec::new(draft)).unwrap();
        validate_horde_tree(&root).expect("written tree with triggers should validate");
        let manifest = parse_app_manifest(&resolve_manifest_path(&root)).unwrap();
        assert_eq!(manifest.triggers, expected);
    }

    #[test]
    fn rejects_invalid_horde_id() {
        let mut draft = minimal_linear_draft();
        draft.id = "../evil".into();
        assert!(validate_draft(&draft).is_err());
    }

    #[test]
    fn write_dag_fixture_emits_edges_and_validates() {
        let dir = tempdir().unwrap();
        let mut draft = minimal_dag_draft();
        normalize_draft(&mut draft);
        validate_draft(&draft).expect("DAG fixture should validate");
        let spec = HordeBirthSpec::new(draft);
        let root = write_horde_tree(dir.path(), &spec).unwrap();
        validate_horde_tree(&root).expect("written DAG tree should validate");
        let body = fs::read_to_string(root.join("horde.md")).unwrap();
        assert!(body.contains("[[edges]]"));
        assert!(body.contains("from = \"ingest\""));
        let join_agent = fs::read_to_string(root.join("agents/join.md")).unwrap();
        assert!(join_agent.contains("@step:branch-a@"));
        assert!(join_agent.contains("@step:branch-b@"));
    }

    #[test]
    fn write_linear_fixture_omits_edges() {
        let dir = tempdir().unwrap();
        let spec = HordeBirthSpec::new(minimal_linear_draft());
        let root = write_horde_tree(dir.path(), &spec).unwrap();
        let body = fs::read_to_string(root.join("horde.md")).unwrap();
        assert!(!body.contains("[[edges]]"));
        validate_horde_tree(&root).expect("linear tree should validate");
    }

    #[test]
    fn coder_example_validates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/coder");
        if !root.join("horde.md").is_file() {
            return;
        }
        validate_horde_tree(&root).expect("examples/coder should validate");
        let manifest = parse_app_manifest(&resolve_manifest_path(&root)).unwrap();
        let edge_slice = if manifest.edges.is_empty() {
            None
        } else {
            Some(manifest.edges.as_slice())
        };
        let graph = crate::resolve_execution_graph(&manifest.pipeline, edge_slice).unwrap();
        assert_eq!(graph.layers[0], vec!["ingest".to_string()]);
        assert_eq!(graph.layers[1].len(), 2);
        assert!(graph.layers[1].contains(&"warmup".to_string()));
        assert!(graph.layers[1].contains(&"todo-plan".to_string()));
        assert_eq!(graph.layers[2], vec!["adjust".to_string()]);
        assert_eq!(graph.layers[9], vec!["deliver".to_string()]);
        assert_eq!(graph.layers.len(), 10);
    }
}
