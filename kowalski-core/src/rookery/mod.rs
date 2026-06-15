//! **Rookery** — horde builder: validate drafts and write markdown-native horde trees.
//!
//! 1.3.0 supports **linear** pipelines only (`horde.md` `pipeline = [...]`). DAG / `edges[]` are 1.4.0+.

mod draft_parse;
mod fixture;
mod normalize;
mod repair;
mod types;
mod validate;
mod writer;

pub use draft_parse::{extract_json_block, parse_draft_from_assistant};
pub use fixture::minimal_linear_draft;
pub use normalize::{
    default_output_for_penguin, normalize_draft, normalize_penguin_output, output_looks_invalid,
    slugify_horde_id,
};
pub use repair::repair_horde_tree_outputs;
pub use types::{HordeBirthSpec, PenguinSpec, RookeryDraft};
pub use validate::{
    validate_draft, validate_horde_id, validate_horde_tree, validate_step_name,
    validate_workdir_relative_path,
};
pub use writer::{horde_root_path, write_horde_tree};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown_pipeline::{parse_app_manifest, parse_stage_agent, resolve_manifest_path};
    use std::fs;
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

    #[test]
    fn rejects_invalid_horde_id() {
        let mut draft = minimal_linear_draft();
        draft.id = "../evil".into();
        assert!(validate_draft(&draft).is_err());
    }
}
