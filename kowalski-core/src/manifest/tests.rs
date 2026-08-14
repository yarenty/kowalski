//! Manifest validation matrix, schema-asset conformance, and horde-dir round-trips.

use crate::horde_graph::HordeEdge;
use crate::horde_trigger::HordeTrigger;
use crate::manifest::*;
use crate::rookery::{RookeryDraft, minimal_dag_draft};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn example_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../examples")
        .join(name)
}

fn schema_validate(manifest: &WorkflowManifest) -> Result<(), String> {
    let validator = jsonschema::validator_for(workflow_manifest_schema()).unwrap();
    let value = serde_json::to_value(manifest).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// DAG draft with a conditional retry loop, triggers, tools, and operator inputs —
/// exercises every manifest field the converters map.
fn loop_dag_draft() -> RookeryDraft {
    let mut draft = minimal_dag_draft();
    draft.version = Some("1.2.3".into());
    draft.edges.push(HordeEdge {
        from: "join".into(),
        to: "branch-a".into(),
        when: Some("fail".into()),
        max_loops: Some(2),
    });
    draft.triggers.push(HordeTrigger {
        cron: Some("*/5 * * * *".into()),
        watch: None,
        webhook: None,
        enabled: false,
        overlap: "queue".into(),
        input: BTreeMap::from([("task_spec".into(), "scheduled".into())]),
        prompt: Some("Cron run at {{trigger.time}}".into()),
    });
    let branch_a = draft
        .penguins
        .iter_mut()
        .find(|p| p.name == "branch-a")
        .unwrap();
    branch_a.tool_ids = vec!["web_search".into(), "fs_read".into()];
    branch_a.model_id = Some("qwen2.5:7b".into());
    draft
}

#[test]
fn validation_rejects_duplicate_step_ids() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    let dup = manifest.steps[0].clone();
    manifest.steps.push(dup);
    let err = validate_manifest(&manifest).unwrap_err().to_string();
    assert!(err.contains("duplicate step id"), "{err}");
}

#[test]
fn validation_rejects_dangling_edge() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    manifest.edges.push(HordeEdge {
        from: "join".into(),
        to: "no-such-step".into(),
        when: None,
        max_loops: None,
    });
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn validation_rejects_pipeline_gap() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    manifest.pipeline.pop();
    let err = validate_manifest(&manifest).unwrap_err().to_string();
    assert!(err.contains("not listed in pipeline"), "{err}");
}

#[test]
fn publish_requires_prompt_on_llm_steps_but_draft_does_not() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    let step = manifest
        .steps
        .iter_mut()
        .find(|s| s.id == "branch-a")
        .unwrap();
    step.agent.as_mut().unwrap().system_prompt = String::new();
    let err = validate_manifest(&manifest).unwrap_err().to_string();
    assert!(err.contains("requires agent.system_prompt"), "{err}");
    validate_manifest_with(&manifest, ManifestStrictness::Draft)
        .expect("draft strictness allows missing prompts");
}

#[test]
fn validation_rejects_kind_config_mismatch() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    let step = manifest
        .steps
        .iter_mut()
        .find(|s| s.id == "branch-a")
        .unwrap();
    step.kowalski
        .get_or_insert_with(Default::default)
        .verify_command = Some("cargo test".into());
    let err = validate_manifest(&manifest).unwrap_err().to_string();
    assert!(err.contains("only valid on kind `verify`"), "{err}");
}

#[test]
fn validation_rejects_bad_schema_version_and_identifier() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    manifest.schema_version = "2.0".into();
    manifest.identifier = "Bad_Id".into();
    let err = validate_manifest(&manifest).unwrap_err().to_string();
    assert!(err.contains("unsupported schema_version"), "{err}");
    assert!(err.contains("identifier"), "{err}");
}

#[test]
fn unknown_kind_is_a_warning_not_an_error() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    manifest.steps[0].kind = "exotic".into();
    // Keep the config exclusive checks quiet: exotic kind has no agent requirement.
    let warnings = validate_manifest(&manifest).expect("unknown kind must not be fatal");
    assert!(
        warnings.iter().any(|w| w.contains("unknown kind `exotic`")),
        "{warnings:?}"
    );
}

#[test]
fn deny_unknown_fields_is_enforced() {
    let mut value = serde_json::to_value(manifest_from_draft(&loop_dag_draft())).unwrap();
    value["surprise"] = serde_json::json!(true);
    assert!(serde_json::from_value::<WorkflowManifest>(value).is_err());
}

#[test]
fn schema_asset_validates_generated_manifests() {
    for name in ["coder", "knowledge-compiler"] {
        let manifest = horde_dir_to_manifest(&example_root(name)).unwrap();
        schema_validate(&manifest).unwrap_or_else(|e| panic!("{name}: {e}"));
        validate_manifest(&manifest).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    let manifest = manifest_from_draft(&loop_dag_draft());
    schema_validate(&manifest).expect("dag fixture manifest must match the schema asset");
}

#[test]
fn schema_asset_rejects_unknown_fields_and_bad_kind() {
    let validator = jsonschema::validator_for(workflow_manifest_schema()).unwrap();
    let mut value = serde_json::to_value(manifest_from_draft(&loop_dag_draft())).unwrap();
    value["surprise"] = serde_json::json!(true);
    assert!(!validator.is_valid(&value));
    let mut value = serde_json::to_value(manifest_from_draft(&loop_dag_draft())).unwrap();
    value["steps"][0]["kind"] = serde_json::json!("exotic");
    assert!(!validator.is_valid(&value));
}

/// dir → manifest → dir preserves semantics: re-exporting the regenerated tree yields
/// the same manifest (prompts inlined, edges/triggers/inputs/tools verbatim).
#[test]
fn example_dirs_round_trip_through_manifest() {
    for name in ["coder", "knowledge-compiler"] {
        let root = example_root(name);
        let manifest = horde_dir_to_manifest(&root).unwrap();
        validate_manifest(&manifest).unwrap_or_else(|e| panic!("{name}: {e}"));

        let tmp = tempfile::tempdir().unwrap();
        let regenerated = write_manifest_tree(tmp.path(), &manifest, false).unwrap();
        let manifest2 = horde_dir_to_manifest(&regenerated).unwrap();
        assert_eq!(
            manifest, manifest2,
            "{name}: dir → manifest → dir must be stable"
        );
        assert_eq!(
            manifest.edges, manifest2.edges,
            "{name}: DAG edges must be preserved exactly"
        );
    }
}

/// manifest → dir → manifest is byte-stable (serialized JSON equality).
#[test]
fn dag_fixture_round_trips_byte_stable() {
    let manifest = manifest_from_draft(&loop_dag_draft());
    validate_manifest(&manifest).unwrap();
    schema_validate(&manifest).unwrap();

    let tmp = tempfile::tempdir().unwrap();
    let root = write_manifest_tree(tmp.path(), &manifest, false).unwrap();
    let manifest2 = horde_dir_to_manifest(&root).unwrap();
    assert_eq!(
        serde_json::to_string_pretty(&manifest).unwrap(),
        serde_json::to_string_pretty(&manifest2).unwrap()
    );
    let loop_edge = manifest2
        .edges
        .iter()
        .find(|e| e.from == "join" && e.to == "branch-a")
        .expect("loop-back edge survives");
    assert_eq!(loop_edge.when.as_deref(), Some("fail"));
    assert_eq!(loop_edge.max_loops, Some(2));
    let trigger = &manifest2.kowalski.as_ref().unwrap().triggers[0];
    assert_eq!(trigger.overlap, "queue");
    assert!(!trigger.enabled);
    assert_eq!(
        trigger.input.get("task_spec").map(String::as_str),
        Some("scheduled")
    );
}
