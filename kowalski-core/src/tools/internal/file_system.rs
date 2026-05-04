//! **Internal filesystem** — small, synchronous helpers for bounded read/write/list/copy.
//!
//! These are **not** a full virtual FS for agents: no glob-from-arbitrary-root without caller checks.
//! Prefer **MCP** for vault-wide indexing, git operations, or cross-machine sync.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Default cap for [`read_file_bounded`] when used from ingest-style callers.
pub const DEFAULT_MAX_READ_BYTES: usize = 256 * 1024;

/// Marker for future `Tool` registration (`internal_fs_read`, etc.).
#[derive(Debug, Clone, Copy, Default)]
pub struct FileSystemInternalModule;

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}

pub fn is_file(path: &Path) -> bool {
    path.is_file()
}

pub fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

/// File size in bytes, or error if missing / not a file.
pub fn file_len(path: &Path) -> Result<u64, String> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err("path is not a regular file".into());
    }
    Ok(m.len())
}

/// Read entire file as UTF-8 if `metadata().len() <= max_bytes`; otherwise error (no partial read that hides truncation).
pub fn read_file_bounded(path: &Path, max_bytes: usize) -> Result<String, String> {
    let len = file_len(path)?;
    if len > max_bytes as u64 {
        return Err(format!(
            "file size {} exceeds max_bytes {}",
            len, max_bytes
        ));
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Read at most `max_bytes` from the start of a file (binary-safe slice, then UTF-8 lossy is wrong for binary — use for text only).
pub fn read_file_prefix(path: &Path, max_bytes: usize) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; max_bytes];
    let n = f.read(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(n);
    String::from_utf8(buf).map_err(|e| format!("invalid UTF-8: {}", e))
}

/// Write `contents` to `path`, creating parent directories.
pub fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, contents.as_bytes()).map_err(|e| e.to_string())
}

/// Append UTF-8 to `path` (creates file if missing).
pub fn append_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(contents.as_bytes()).map_err(|e| e.to_string())
}

/// `fs::create_dir_all`
pub fn mkdir_all(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())
}

/// Remove a single file (not a directory). Intended for scratch cleanup in controlled paths.
pub fn remove_file(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
}

/// Copy file to `dst` (parent dirs created).
pub fn copy_file(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

/// Rename or move `from` → `to` (same filesystem).
pub fn rename(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::rename(from, to).map_err(|e| e.to_string())
}

/// Top-level directory entries (names only), sorted, capped at `max_entries`.
pub fn list_dir_names(path: &Path, max_entries: usize) -> Result<Vec<String>, String> {
    if !path.is_dir() {
        return Err("path is not a directory".into());
    }
    let mut names = Vec::new();
    for e in fs::read_dir(path).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        names.push(e.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    if names.len() > max_entries {
        names.truncate(max_entries);
    }
    Ok(names)
}

/// One-level listing with `is_dir` hint: `(name, is_directory)`.
pub fn list_dir_entries(path: &Path, max_entries: usize) -> Result<Vec<(String, bool)>, String> {
    if !path.is_dir() {
        return Err("path is not a directory".into());
    }
    let mut out = Vec::new();
    for e in fs::read_dir(path).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        let is_d = p.is_dir();
        out.push((e.file_name().to_string_lossy().into_owned(), is_d));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out.truncate(max_entries);
    Ok(out)
}

/// Canonicalize if possible; on error returns the original path as [`PathBuf`].
pub fn try_canonicalize(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn write_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.txt");
        write_file(&p, "hello").unwrap();
        assert_eq!(read_file_bounded(&p, 1024).unwrap(), "hello");
    }

    #[test]
    fn read_too_large_errors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.txt");
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(&vec![b'x'; 100]).unwrap();
        assert!(read_file_bounded(&p, 50).is_err());
    }

    #[test]
    fn list_dir_names_sorted() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b"), "").unwrap();
        fs::write(dir.path().join("a"), "").unwrap();
        let n = list_dir_names(dir.path(), 10).unwrap();
        assert_eq!(n, vec!["a".to_string(), "b".to_string()]);
    }
}
