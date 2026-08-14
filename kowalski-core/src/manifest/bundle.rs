//! Portable workflow bundle: a zip file carrying `manifest.json` plus optional
//! `assets/` files (icons, ui samples). Native bundles are named
//! `<identifier>-<version>.kwf.zip`; import also accepts the `.bbwf.zip` extension —
//! an identical format kept as an interop alias for externally produced bundles.
//!
//! Bundles are export artifacts, not backups: by construction they never contain
//! credentials, run history or state, watched-directory contents, or server-side
//! trigger-override state. Import treats the zip as untrusted (entry allowlist,
//! zip-slip guard, size caps), validates the manifest, and lands the horde as a
//! draft directory that is never auto-armed: every imported trigger declaration is
//! rewritten to `enabled = false` so the operator re-enables deliberately.

use crate::error::KowalskiError;
use crate::manifest::convert::{horde_dir_to_manifest, manifest_to_draft};
use crate::manifest::model::{
    BUILTIN_TOOL_PROVIDER, MANIFEST_SCHEMA_MAJOR, MANIFEST_SCHEMA_VERSION, WorkflowManifest,
};
use crate::manifest::validate::{is_known_step_kind, parse_schema_version, validate_manifest};
use crate::rookery::{HordeBirthSpec, write_horde_tree};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Native bundle extension (`<identifier>-<version>.kwf.zip`).
pub const BUNDLE_EXTENSION: &str = "kwf.zip";
/// Accepted alias extension on import (identical format; interop with externally
/// produced workflow bundles).
pub const BUNDLE_EXTENSION_ALIAS: &str = "bbwf.zip";
/// The manifest entry every bundle must carry, byte-identical to the stored manifest.
pub const BUNDLE_MANIFEST_ENTRY: &str = "manifest.json";
/// Prefix for optional asset entries.
pub const BUNDLE_ASSETS_PREFIX: &str = "assets/";

/// Cap on the bundle file itself (compressed bytes on disk).
pub const MAX_BUNDLE_FILE_BYTES: u64 = 32 * 1024 * 1024;
/// Cap on any single entry's uncompressed bytes.
pub const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
/// Cap on the sum of uncompressed bytes across all entries.
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// Cap on the number of entries.
pub const MAX_BUNDLE_ENTRIES: usize = 256;

/// A `schema_version` MINOR migration: upgrades a raw manifest value from one minor
/// version to the next, returning a human-readable note of what changed.
pub type ManifestMigration = fn(&mut Value) -> Result<String, KowalskiError>;

/// Registered MINOR migrations: the entry `(m, f)` upgrades minor `m` to `m + 1`.
/// Empty while the current contract is the first minor of its major.
const MINOR_MIGRATIONS: &[(u32, ManifestMigration)] = &[];

/// What the importing deployment knows, for [`PortabilityReport`] checks.
/// `None` lists skip that check (the caller cannot enumerate the resource).
#[derive(Debug, Clone)]
pub struct PortabilityContext {
    /// Tool providers this deployment can bind.
    pub known_providers: Vec<String>,
    /// Locally available tool ids for [`BUILTIN_TOOL_PROVIDER`] bindings.
    pub known_tool_ids: Option<Vec<String>>,
    /// Locally available model ids (pinned models are checked against these).
    pub available_models: Option<Vec<String>>,
}

impl Default for PortabilityContext {
    fn default() -> Self {
        Self {
            known_providers: vec![BUILTIN_TOOL_PROVIDER.to_string()],
            known_tool_ids: None,
            available_models: None,
        }
    }
}

/// What the target machine is missing or should know about an imported workflow.
/// Reported, never fatal — the horde still lands as a draft.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PortabilityReport {
    /// Step kinds this deployment cannot execute.
    pub unknown_step_kinds: Vec<String>,
    /// Tool-binding providers this deployment cannot bind.
    pub unknown_tool_providers: Vec<String>,
    /// Built-in tool ids not available locally.
    pub unknown_tool_ids: Vec<String>,
    /// Pinned models not available locally.
    pub unresolved_models: Vec<String>,
    /// Non-fatal validation warnings (unknown kinds, trigger notes, …).
    pub warnings: Vec<String>,
    /// Notes from applied MINOR schema migrations.
    pub migrations: Vec<String>,
    /// How many trigger declarations were rewritten to `enabled = false`.
    pub triggers_disabled: usize,
}

impl PortabilityReport {
    /// True when the workflow is fully portable to this deployment (informational
    /// fields — applied migrations, disabled triggers — do not count).
    pub fn is_empty(&self) -> bool {
        self.unknown_step_kinds.is_empty()
            && self.unknown_tool_providers.is_empty()
            && self.unknown_tool_ids.is_empty()
            && self.unresolved_models.is_empty()
            && self.warnings.is_empty()
    }
}

/// Result of a successful bundle import.
#[derive(Debug)]
pub struct BundleImport {
    /// The written draft horde directory.
    pub horde_root: PathBuf,
    /// The imported manifest (triggers already rewritten to `enabled = false`).
    pub manifest: WorkflowManifest,
    pub report: PortabilityReport,
}

/// Result of a dry-run bundle inspection: everything an import would do — guards,
/// migration, validation, trigger rewrite, portability report — without writing.
#[derive(Debug)]
pub struct BundleInspection {
    /// The manifest as an import would land it (triggers rewritten to `enabled = false`).
    pub manifest: WorkflowManifest,
    pub report: PortabilityReport,
}

/// A bundle read into memory with its manifest migrated, validated, and reported on.
struct LoadedBundle {
    manifest: WorkflowManifest,
    assets: Vec<(PathBuf, Vec<u8>)>,
    report: PortabilityReport,
}

/// Canonical bundle file name for a manifest: `<identifier>-<version>.kwf.zip`.
pub fn bundle_file_name(manifest: &WorkflowManifest) -> String {
    format!(
        "{}-{}.{}",
        manifest.identifier, manifest.version, BUNDLE_EXTENSION
    )
}

/// Export a manifest (plus optional assets directory) as a bundle under `dest_dir`.
/// Validates at publish strictness; returns the written bundle path.
pub fn export_bundle(
    manifest: &WorkflowManifest,
    assets_root: Option<&Path>,
    dest_dir: &Path,
) -> Result<PathBuf, KowalskiError> {
    validate_manifest(manifest)?;
    fs::create_dir_all(dest_dir)
        .map_err(|e| KowalskiError::Validation(format!("create {}: {e}", dest_dir.display())))?;
    let bundle_path = dest_dir.join(bundle_file_name(manifest));
    let file = fs::File::create(&bundle_path)
        .map_err(|e| KowalskiError::Validation(format!("create {}: {e}", bundle_path.display())))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let manifest_json = serde_json::to_string_pretty(manifest)
        .map_err(|e| KowalskiError::Validation(format!("serialize manifest: {e}")))?;
    zip.start_file(BUNDLE_MANIFEST_ENTRY, options)
        .and_then(|_| zip.write_all(manifest_json.as_bytes()).map_err(Into::into))
        .map_err(|e| KowalskiError::Validation(format!("write {BUNDLE_MANIFEST_ENTRY}: {e}")))?;

    if let Some(assets) = assets_root.filter(|p| p.is_dir()) {
        for rel in collect_asset_files(assets, Path::new(""))? {
            let bytes = fs::read(assets.join(&rel)).map_err(|e| {
                KowalskiError::Validation(format!("read asset {}: {e}", rel.display()))
            })?;
            let entry = format!(
                "{BUNDLE_ASSETS_PREFIX}{}",
                rel.to_string_lossy().replace('\\', "/")
            );
            zip.start_file(&entry, options)
                .and_then(|_| zip.write_all(&bytes).map_err(Into::into))
                .map_err(|e| KowalskiError::Validation(format!("write {entry}: {e}")))?;
        }
    }

    zip.finish()
        .map_err(|e| KowalskiError::Validation(format!("finish bundle: {e}")))?;
    Ok(bundle_path)
}

/// Export a horde directory as a bundle: manifest projection plus the directory's
/// `assets/` subtree when present. Run state, credentials, and operator override
/// files never enter the bundle — export writes only the manifest and assets.
pub fn export_horde_dir_bundle(
    horde_root: &Path,
    dest_dir: &Path,
) -> Result<PathBuf, KowalskiError> {
    let manifest = horde_dir_to_manifest(horde_root)?;
    let assets = horde_root.join("assets");
    export_bundle(&manifest, assets.is_dir().then_some(assets.as_path()), dest_dir)
}

/// Import a bundle as a draft horde directory under `dest_root`.
///
/// The zip is untrusted: entries outside `manifest.json` / `assets/` are rejected,
/// as are traversal paths, symlinks, and anything beyond the size caps. The manifest
/// is migrated (older MINOR) or rejected (different MAJOR), validated, and written
/// through the normal horde writer with every trigger declaration rewritten to
/// `enabled = false`. Portability problems are returned in the report, not raised.
pub fn import_bundle(
    bundle_path: &Path,
    dest_root: &Path,
    overwrite: bool,
    context: &PortabilityContext,
) -> Result<BundleImport, KowalskiError> {
    let loaded = load_bundle(bundle_path, context)?;
    let draft = manifest_to_draft(&loaded.manifest)?;
    let horde_root = write_horde_tree(
        dest_root,
        &HordeBirthSpec::new(draft).with_overwrite(overwrite),
    )?;
    for (rel, bytes) in &loaded.assets {
        let path = horde_root.join("assets").join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KowalskiError::Validation(format!("create {}: {e}", parent.display()))
            })?;
        }
        fs::write(&path, bytes)
            .map_err(|e| KowalskiError::Validation(format!("write {}: {e}", path.display())))?;
    }
    Ok(BundleImport {
        horde_root,
        manifest: loaded.manifest,
        report: loaded.report,
    })
}

/// Dry-run an import: run every gate an import runs (extension, untrusted-zip
/// guards, schema_version migration, validation, trigger rewrite) and return the
/// resulting manifest plus portability report without writing anything.
pub fn inspect_bundle(
    bundle_path: &Path,
    context: &PortabilityContext,
) -> Result<BundleInspection, KowalskiError> {
    let loaded = load_bundle(bundle_path, context)?;
    Ok(BundleInspection {
        manifest: loaded.manifest,
        report: loaded.report,
    })
}

/// Shared import front half: extension + size gates, untrusted-zip entry walk,
/// manifest migration/validation, trigger rewrite, portability report.
fn load_bundle(
    bundle_path: &Path,
    context: &PortabilityContext,
) -> Result<LoadedBundle, KowalskiError> {
    let name = bundle_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !name.ends_with(&format!(".{BUNDLE_EXTENSION}"))
        && !name.ends_with(&format!(".{BUNDLE_EXTENSION_ALIAS}"))
    {
        return Err(KowalskiError::Validation(format!(
            "bundle `{name}` must use the .{BUNDLE_EXTENSION} extension (or the .{BUNDLE_EXTENSION_ALIAS} interop alias)"
        )));
    }
    let meta = fs::metadata(bundle_path)
        .map_err(|e| KowalskiError::Validation(format!("read {}: {e}", bundle_path.display())))?;
    if meta.len() > MAX_BUNDLE_FILE_BYTES {
        return Err(KowalskiError::Validation(format!(
            "bundle is {} bytes, above the {MAX_BUNDLE_FILE_BYTES}-byte cap",
            meta.len()
        )));
    }

    let file = fs::File::open(bundle_path)
        .map_err(|e| KowalskiError::Validation(format!("open {}: {e}", bundle_path.display())))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| KowalskiError::Validation(format!("not a valid zip bundle: {e}")))?;
    if archive.len() > MAX_BUNDLE_ENTRIES {
        return Err(KowalskiError::Validation(format!(
            "bundle has {} entries, above the {MAX_BUNDLE_ENTRIES}-entry cap",
            archive.len()
        )));
    }

    let mut manifest_bytes: Option<Vec<u8>> = None;
    let mut assets: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut total: u64 = 0;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| KowalskiError::Validation(format!("read bundle entry {index}: {e}")))?;
        let raw_name = entry.name().to_string();
        let safe = entry.enclosed_name().ok_or_else(|| {
            KowalskiError::Validation(format!("bundle entry `{raw_name}` has an unsafe path"))
        })?;
        if entry
            .unix_mode()
            .is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(KowalskiError::Validation(format!(
                "bundle entry `{raw_name}` is a symlink, which bundles must not contain"
            )));
        }
        if entry.is_dir() {
            continue;
        }
        let is_manifest = raw_name == BUNDLE_MANIFEST_ENTRY;
        if !is_manifest && !raw_name.starts_with(BUNDLE_ASSETS_PREFIX) {
            return Err(KowalskiError::Validation(format!(
                "bundle entry `{raw_name}` is not allowed (only {BUNDLE_MANIFEST_ENTRY} and {BUNDLE_ASSETS_PREFIX}* may appear)"
            )));
        }
        let bytes = read_capped(&mut entry, &raw_name)?;
        total += bytes.len() as u64;
        if total > MAX_TOTAL_UNCOMPRESSED_BYTES {
            return Err(KowalskiError::Validation(format!(
                "bundle expands past the {MAX_TOTAL_UNCOMPRESSED_BYTES}-byte total cap"
            )));
        }
        if is_manifest {
            manifest_bytes = Some(bytes);
        } else {
            let rel = safe
                .strip_prefix(BUNDLE_ASSETS_PREFIX.trim_end_matches('/'))
                .map_err(|_| {
                    KowalskiError::Validation(format!("bundle entry `{raw_name}` has an unsafe path"))
                })?
                .to_path_buf();
            assets.push((rel, bytes));
        }
    }

    let manifest_bytes = manifest_bytes.ok_or_else(|| {
        KowalskiError::Validation(format!("bundle has no {BUNDLE_MANIFEST_ENTRY} entry"))
    })?;
    let mut value: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| KowalskiError::Validation(format!("{BUNDLE_MANIFEST_ENTRY}: {e}")))?;
    let migrations = migrate_manifest_value(&mut value)?;
    let mut manifest: WorkflowManifest = serde_json::from_value(value)
        .map_err(|e| KowalskiError::Validation(format!("{BUNDLE_MANIFEST_ENTRY}: {e}")))?;

    // Imports are never auto-armed: the declaration itself is rewritten so the file
    // stays authoritative (operator trigger overrides are server state, not ours).
    let mut triggers_disabled = 0usize;
    if let Some(ext) = manifest.kowalski.as_mut() {
        for trigger in ext.triggers.iter_mut().filter(|t| t.enabled) {
            trigger.enabled = false;
            triggers_disabled += 1;
        }
    }

    let warnings = validate_manifest(&manifest)?;
    let mut report = portability_report(&manifest, &warnings, context);
    report.migrations = migrations;
    report.triggers_disabled = triggers_disabled;
    Ok(LoadedBundle {
        manifest,
        assets,
        report,
    })
}

/// What this deployment is missing to run the manifest, from validation warnings
/// plus tool/model checks against the supplied [`PortabilityContext`].
pub fn portability_report(
    manifest: &WorkflowManifest,
    warnings: &[String],
    context: &PortabilityContext,
) -> PortabilityReport {
    let mut kinds = BTreeSet::new();
    let mut providers = BTreeSet::new();
    let mut tools = BTreeSet::new();
    let mut models = BTreeSet::new();
    for step in &manifest.steps {
        if !is_known_step_kind(&step.kind) {
            kinds.insert(step.kind.clone());
        }
        for binding in &step.tool_bindings {
            if !context.known_providers.contains(&binding.provider) {
                providers.insert(binding.provider.clone());
            } else if binding.provider == BUILTIN_TOOL_PROVIDER
                && let Some(known) = &context.known_tool_ids
            {
                for tool in binding.tools.iter().flatten() {
                    if !known.contains(tool) {
                        tools.insert(tool.clone());
                    }
                }
            }
        }
        if let Some(available) = &context.available_models
            && let Some(model) = step.agent.as_ref().and_then(|a| a.model.as_ref())
            && !available.contains(model)
        {
            models.insert(model.clone());
        }
    }
    PortabilityReport {
        unknown_step_kinds: kinds.into_iter().collect(),
        unknown_tool_providers: providers.into_iter().collect(),
        unknown_tool_ids: tools.into_iter().collect(),
        unresolved_models: models.into_iter().collect(),
        warnings: warnings.to_vec(),
        migrations: Vec::new(),
        triggers_disabled: 0,
    }
}

/// Bring a raw manifest value to the current schema_version: different MAJOR is
/// rejected, newer MINOR is rejected (this deployment is too old), older MINOR walks
/// the registered migration chain. Returns the applied migration notes.
fn migrate_manifest_value(value: &mut Value) -> Result<Vec<String>, KowalskiError> {
    let (_, current_minor) = parse_schema_version(MANIFEST_SCHEMA_VERSION)
        .expect("MANIFEST_SCHEMA_VERSION is MAJOR.MINOR");
    apply_minor_migrations(value, current_minor, MINOR_MIGRATIONS)
}

/// Migration walk, target-and-table-injectable so tests can prove the path even
/// while the shipped table is empty.
pub(crate) fn apply_minor_migrations(
    value: &mut Value,
    current_minor: u32,
    migrations: &[(u32, ManifestMigration)],
) -> Result<Vec<String>, KowalskiError> {
    let declared = value
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            KowalskiError::Validation("manifest has no schema_version string".into())
        })?;
    let (major, minor) =
        parse_schema_version(declared).map_err(KowalskiError::Validation)?;
    if major != MANIFEST_SCHEMA_MAJOR {
        return Err(KowalskiError::Validation(format!(
            "bundle schema_version `{declared}` has unsupported MAJOR (supported: {MANIFEST_SCHEMA_MAJOR})"
        )));
    }
    if minor > current_minor {
        return Err(KowalskiError::Validation(format!(
            "bundle schema_version `{declared}` is newer than this deployment supports ({MANIFEST_SCHEMA_MAJOR}.{current_minor}); upgrade before importing"
        )));
    }
    let mut notes = Vec::new();
    for step in minor..current_minor {
        let (_, migration) = migrations
            .iter()
            .find(|(from, _)| *from == step)
            .ok_or_else(|| {
                KowalskiError::Validation(format!(
                    "no migration registered from schema_version {MANIFEST_SCHEMA_MAJOR}.{step}"
                ))
            })?;
        notes.push(migration(value)?);
    }
    if minor < current_minor {
        value["schema_version"] =
            Value::String(format!("{MANIFEST_SCHEMA_MAJOR}.{current_minor}"));
    }
    Ok(notes)
}

fn read_capped(entry: &mut impl Read, name: &str) -> Result<Vec<u8>, KowalskiError> {
    let mut bytes = Vec::new();
    entry
        .take(MAX_ENTRY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| KowalskiError::Validation(format!("read bundle entry `{name}`: {e}")))?;
    if bytes.len() as u64 > MAX_ENTRY_BYTES {
        return Err(KowalskiError::Validation(format!(
            "bundle entry `{name}` exceeds the {MAX_ENTRY_BYTES}-byte entry cap"
        )));
    }
    Ok(bytes)
}

fn collect_asset_files(root: &Path, rel: &Path) -> Result<Vec<PathBuf>, KowalskiError> {
    let dir = root.join(rel);
    let mut files = Vec::new();
    let entries = fs::read_dir(&dir)
        .map_err(|e| KowalskiError::Validation(format!("read {}: {e}", dir.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| KowalskiError::Validation(format!("read {}: {e}", dir.display())))?;
        let child = rel.join(entry.file_name());
        let meta = fs::symlink_metadata(entry.path())
            .map_err(|e| KowalskiError::Validation(format!("stat {}: {e}", child.display())))?;
        if meta.is_dir() {
            files.extend(collect_asset_files(root, &child)?);
        } else if meta.is_file() {
            files.push(child);
        }
        // Symlinks are skipped: bundles carry only regular files.
    }
    files.sort();
    Ok(files)
}
