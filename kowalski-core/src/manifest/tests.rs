//! Manifest validation matrix, schema-asset conformance, horde-dir round-trips,
//! and portable-bundle export/import (zip guards, portability report, migrations).

use crate::horde_graph::HordeEdge;
use crate::horde_trigger::HordeTrigger;
use crate::manifest::bundle::apply_minor_migrations;
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

// ---------------------------------------------------------------------------
// Portable bundle (.kwf.zip)
// ---------------------------------------------------------------------------

/// DAG manifest exercising every portability dimension: an enabled trigger, a
/// foreign tool provider, builtin tools, and a pinned model.
fn portability_manifest() -> WorkflowManifest {
    let mut draft = loop_dag_draft();
    draft.triggers[0].enabled = true;
    let mut manifest = manifest_from_draft(&draft);
    let step = manifest
        .steps
        .iter_mut()
        .find(|s| s.id == "branch-b")
        .unwrap();
    step.tool_bindings.push(ToolBinding {
        provider: "acme-tools".into(),
        tools: Some(vec!["acme_search".into()]),
        overrides: None,
    });
    manifest
}

fn write_raw_bundle(path: &PathBuf, entries: &[(&str, &[u8])]) {
    use std::io::Write;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn valid_manifest_json() -> Vec<u8> {
    serde_json::to_vec_pretty(&manifest_from_draft(&loop_dag_draft())).unwrap()
}

#[test]
fn bundle_round_trip_on_example_is_equivalent_and_report_clean() {
    let root = example_root("coder");
    let manifest = horde_dir_to_manifest(&root).unwrap();
    let tmp = tempfile::tempdir().unwrap();

    let bundle = export_horde_dir_bundle(&root, &tmp.path().join("out")).unwrap();
    assert_eq!(
        bundle.file_name().unwrap().to_string_lossy(),
        bundle_file_name(&manifest)
    );
    assert!(bundle.to_string_lossy().ends_with(".kwf.zip"));

    let imported = import_bundle(
        &bundle,
        &tmp.path().join("hordes"),
        false,
        &PortabilityContext::default(),
    )
    .unwrap();
    assert!(
        imported.report.is_empty(),
        "same-machine report must be empty: {:?}",
        imported.report
    );
    let manifest2 = horde_dir_to_manifest(&imported.horde_root).unwrap();
    // equivalent except that an import runs every step isolated
    let mut expected = manifest.clone();
    for step in expected.steps.iter_mut() {
        step.kowalski.get_or_insert_with(Default::default).isolation = Some("process".into());
    }
    assert_eq!(expected, manifest2, "export → import must be equivalent");
}

#[test]
fn bundle_import_reports_gaps_and_disables_triggers() {
    let manifest = portability_manifest();
    let tmp = tempfile::tempdir().unwrap();
    let assets = tmp.path().join("assets-src");
    std::fs::create_dir_all(assets.join("ui")).unwrap();
    std::fs::write(assets.join("icon.svg"), "<svg/>").unwrap();
    std::fs::write(assets.join("ui/sample.json"), "{}").unwrap();

    let bundle = export_bundle(&manifest, Some(&assets), &tmp.path().join("out")).unwrap();
    let context = PortabilityContext {
        known_providers: vec![BUILTIN_TOOL_PROVIDER.into()],
        known_tool_ids: Some(vec!["fs_read".into()]),
        available_models: Some(vec!["llama3:8b".into()]),
    };
    let imported = import_bundle(&bundle, &tmp.path().join("hordes"), false, &context).unwrap();

    let report = &imported.report;
    assert_eq!(report.unknown_tool_providers, vec!["acme-tools".to_string()]);
    assert_eq!(report.unknown_tool_ids, vec!["web_search".to_string()]);
    assert_eq!(report.unresolved_models, vec!["qwen2.5:7b".to_string()]);
    assert_eq!(report.triggers_disabled, 1);
    assert!(!report.is_empty());

    // The declaration itself is rewritten: a fresh parse of the landed dir sees
    // enabled = false, with the rest of the trigger intact.
    let landed = draft_from_horde_dir(&imported.horde_root).unwrap();
    assert!(!landed.triggers[0].enabled);
    assert_eq!(landed.triggers[0].cron.as_deref(), Some("*/5 * * * *"));
    assert!(report.steps_isolated > 0);
    assert!(
        landed.penguins.iter().all(|p| p.isolation.as_deref() == Some("process")),
        "imported steps run in a child process"
    );
    assert!(imported.horde_root.join("assets/icon.svg").is_file());
    assert!(imported.horde_root.join("assets/ui/sample.json").is_file());
}

#[test]
fn bbwf_alias_imports_identically_and_other_extensions_are_rejected() {
    let manifest = portability_manifest();
    let tmp = tempfile::tempdir().unwrap();
    let bundle = export_bundle(&manifest, None, tmp.path()).unwrap();

    let alias = tmp.path().join("interop.bbwf.zip");
    std::fs::copy(&bundle, &alias).unwrap();
    let a = import_bundle(
        &bundle,
        &tmp.path().join("a"),
        false,
        &PortabilityContext::default(),
    )
    .unwrap();
    let b = import_bundle(
        &alias,
        &tmp.path().join("b"),
        false,
        &PortabilityContext::default(),
    )
    .unwrap();
    assert_eq!(a.manifest, b.manifest);
    assert_eq!(a.report, b.report);

    let plain = tmp.path().join("something.zip");
    std::fs::copy(&bundle, &plain).unwrap();
    let err = import_bundle(
        &plain,
        &tmp.path().join("c"),
        false,
        &PortabilityContext::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains(".kwf.zip"), "{err}");
}

#[test]
fn unknown_kind_lands_in_portability_report() {
    let mut manifest = manifest_from_draft(&loop_dag_draft());
    manifest.steps[0].kind = "exotic".into();
    let warnings = validate_manifest(&manifest).unwrap();
    let report = portability_report(&manifest, &warnings, &PortabilityContext::default());
    assert_eq!(report.unknown_step_kinds, vec!["exotic".to_string()]);
    assert!(!report.warnings.is_empty());
}

#[test]
fn zip_traversal_entry_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("evil-0.1.0.kwf.zip");
    write_raw_bundle(
        &path,
        &[
            ("../evil.txt", b"boom".as_slice()),
            ("manifest.json", &valid_manifest_json()),
        ],
    );
    let err = import_bundle(
        &path,
        tmp.path(),
        false,
        &PortabilityContext::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("unsafe path"), "{err}");
}

#[test]
fn unexpected_bundle_entries_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("sneaky-0.1.0.kwf.zip");
    write_raw_bundle(
        &path,
        &[
            ("manifest.json", valid_manifest_json().as_slice()),
            ("db/trigger_overrides.json", b"{}".as_slice()),
        ],
    );
    let err = import_bundle(
        &path,
        tmp.path(),
        false,
        &PortabilityContext::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("not allowed"), "{err}");
}

#[test]
fn oversize_bundle_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("big-0.1.0.kwf.zip");
    let huge = vec![b' '; (MAX_ENTRY_BYTES + 1) as usize];
    write_raw_bundle(&path, &[("assets/huge.bin", huge.as_slice())]);
    let err = import_bundle(
        &path,
        tmp.path(),
        false,
        &PortabilityContext::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("cap"), "{err}");

    let path = tmp.path().join("many-0.1.0.kwf.zip");
    let entries: Vec<(String, &[u8])> = (0..=MAX_BUNDLE_ENTRIES)
        .map(|i| (format!("assets/f{i}"), b"x".as_slice()))
        .collect();
    let borrowed: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), *b))
        .collect();
    write_raw_bundle(&path, &borrowed);
    let err = import_bundle(
        &path,
        tmp.path(),
        false,
        &PortabilityContext::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("entry cap") || err.contains("entries"), "{err}");
}

/// Bundles never carry credentials or server-side operator state: the entry set is
/// exactly the manifest plus assets, and no credential-shaped key exists anywhere in
/// the manifest JSON (keys are audited recursively; prompt *values* are free text).
#[test]
fn bundle_contents_carry_no_secrets_or_server_state() {
    let manifest = portability_manifest();
    let tmp = tempfile::tempdir().unwrap();
    let assets = tmp.path().join("assets-src");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("icon.svg"), "<svg/>").unwrap();
    let bundle = export_bundle(&manifest, Some(&assets), tmp.path()).unwrap();

    let file = std::fs::File::open(&bundle).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut manifest_bytes = Vec::new();
    for i in 0..archive.len() {
        use std::io::Read;
        let mut entry = archive.by_index(i).unwrap();
        let name = entry.name().to_string();
        assert!(
            name == BUNDLE_MANIFEST_ENTRY || name.starts_with(BUNDLE_ASSETS_PREFIX),
            "unexpected bundle entry `{name}`"
        );
        if name == BUNDLE_MANIFEST_ENTRY {
            entry.read_to_end(&mut manifest_bytes).unwrap();
        }
    }

    let value: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);
    const FORBIDDEN: &[&str] = &["token", "secret", "password", "api_key", "credential", "auth"];
    for key in &keys {
        let lower = key.to_lowercase();
        assert!(
            !FORBIDDEN.iter().any(|f| lower.contains(f)),
            "credential-shaped key `{key}` must never serialize into a bundle"
        );
        assert!(
            !lower.contains("override") || key == "overrides",
            "server-side override state must never serialize into a bundle (`{key}`)"
        );
    }
}

fn collect_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                keys.push(k.clone());
                collect_keys(v, keys);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_keys(v, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn schema_version_major_and_minor_handling() {
    // Different MAJOR: rejected outright, end to end through import.
    let mut value = serde_json::to_value(manifest_from_draft(&loop_dag_draft())).unwrap();
    value["schema_version"] = serde_json::json!("2.0");
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("future-0.1.0.kwf.zip");
    write_raw_bundle(
        &path,
        &[("manifest.json", serde_json::to_vec(&value).unwrap().as_slice())],
    );
    let err = import_bundle(
        &path,
        tmp.path(),
        false,
        &PortabilityContext::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("MAJOR"), "{err}");

    // Newer MINOR than this deployment: rejected with an upgrade hint.
    let mut value = serde_json::json!({ "schema_version": "1.5" });
    let err = apply_minor_migrations(&mut value, 0, &[])
        .unwrap_err()
        .to_string();
    assert!(err.contains("newer"), "{err}");

    // Older MINOR: walks the registered chain and stamps the current version.
    fn noop(_v: &mut serde_json::Value) -> Result<String, crate::error::KowalskiError> {
        Ok("1.0 → 1.1: no structural changes".into())
    }
    let mut value = serde_json::json!({ "schema_version": "1.0" });
    let notes = apply_minor_migrations(&mut value, 1, &[(0, noop)]).unwrap();
    assert_eq!(notes, vec!["1.0 → 1.1: no structural changes".to_string()]);
    assert_eq!(value["schema_version"], serde_json::json!("1.1"));

    // A gap in the chain is a hard error, not a silent skip.
    let mut value = serde_json::json!({ "schema_version": "1.0" });
    let err = apply_minor_migrations(&mut value, 1, &[])
        .unwrap_err()
        .to_string();
    assert!(err.contains("no migration registered"), "{err}");
}
