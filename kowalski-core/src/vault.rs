//! Notes vault: a folder of Markdown notes (an Obsidian vault, or any folder) where hordes that
//! opt in (`vault = true` in `horde.md`) leave their delivered note when a run finishes.
//!
//! Notes go into [`VAULT_FOLDER`] inside the vault, named `<date> <title>.md`, with front matter
//! saying where they came from. An existing note is never overwritten: a second note with the
//! same name gets ` (2)`, ` (3)`, … Obsidian picks new files up by itself.

use std::path::{Path, PathBuf};

/// The folder inside the vault that kowalski writes into.
pub const VAULT_FOLDER: &str = "Kowalski";

/// One note for the vault.
#[derive(Debug, Clone)]
pub struct VaultNote<'a> {
    /// Human title (the run's title); also the file name, made safe.
    pub title: &'a str,
    /// The horde that wrote it.
    pub horde: &'a str,
    /// The run it came from.
    pub run_id: &'a str,
    /// `YYYY-MM-DD`.
    pub date: &'a str,
    /// The delivered Markdown, possibly with its own `---` front matter.
    pub body: &'a str,
}

/// Write `note` into `<vault>/Kowalski/` and return the file's path.
pub fn save_note(vault: &Path, note: &VaultNote<'_>) -> std::io::Result<PathBuf> {
    if !vault.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("the notes vault {} is not a folder", vault.display()),
        ));
    }
    let dir = vault.join(VAULT_FOLDER);
    std::fs::create_dir_all(&dir)?;
    let stem = format!("{} {}", note.date, file_safe(note.title));
    let mut path = dir.join(format!("{stem}.md"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}).md"));
        n += 1;
    }
    std::fs::write(&path, with_front_matter(note))?;
    Ok(path)
}

/// A title as a file name: no path separators or characters Obsidian and common file systems
/// refuse, one line, at most 80 characters.
fn file_safe(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '#' | '^' | '[' | ']') || c.is_control() { ' ' } else { c })
        .collect();
    let one_line = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = one_line.chars().take(80).collect();
    let short = short.trim().trim_end_matches('.').trim().to_string();
    if short.is_empty() { "Note".into() } else { short }
}

/// The note's text: its own front matter (if any) kept, with `source`, `horde`, `run`, `created`
/// and a `kowalski` tag added where missing.
fn with_front_matter(note: &VaultNote<'_>) -> String {
    let ours = [
        ("source", "kowalski".to_string()),
        ("horde", note.horde.to_string()),
        ("run", note.run_id.to_string()),
        ("created", note.date.to_string()),
    ];
    let (existing, body) = split_front_matter(note.body);
    let mut lines: Vec<String> = existing.map(|fm| fm.lines().map(str::to_string).collect()).unwrap_or_default();
    let has = |lines: &[String], key: &str| lines.iter().any(|l| l.trim_start().starts_with(&format!("{key}:")));
    for (key, value) in ours {
        if !has(&lines, key) {
            lines.push(format!("{key}: {value}"));
        }
    }
    if !has(&lines, "tags") {
        lines.push("tags:".into());
        lines.push("  - kowalski".into());
    }
    format!("---\n{}\n---\n\n{}\n", lines.join("\n"), body.trim())
}

/// `(front matter without the fences, rest)` when `text` starts with a `---` block. A block the
/// model forgot to close ends at the first line that is not YAML (`key:` or a `- item` list).
pub fn split_front_matter(text: &str) -> (Option<&str>, &str) {
    let Some(rest) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) else {
        return (None, text);
    };
    let yaml_until = |s: &str| {
        let mut end = 0;
        for line in s.split_inclusive('\n') {
            let t = line.trim();
            let key = t.split_once(':').is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
            if t.is_empty() || t == "---" || !(key || t.starts_with("- ")) {
                break;
            }
            end += line.len();
        }
        end
    };
    let end = yaml_until(rest);
    let after = &rest[end..];
    match after.strip_prefix("---") {
        // closed as it should be
        Some(tail) => (Some(rest[..end].trim_end()), tail.trim_start_matches(['\r', '\n'])),
        // left open: the YAML lines are the block, the rest is the note
        None if end > 0 => (Some(rest[..end].trim_end()), after.trim_start_matches(['\r', '\n'])),
        None => (None, text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note<'a>(title: &'a str, body: &'a str) -> VaultNote<'a> {
        VaultNote { title, horde: "knowledge-compiler", run_id: "run-1", date: "2026-10-01", body }
    }

    #[test]
    fn saves_with_front_matter_and_never_overwrites() {
        let vault = tempfile::tempdir().unwrap();
        let a = save_note(vault.path(), &note("What is: Rust? / async", "# Rust\n\nbody")).unwrap();
        assert_eq!(a, vault.path().join("Kowalski/2026-10-01 What is Rust async.md"));
        let text = std::fs::read_to_string(&a).unwrap();
        assert!(text.starts_with("---\nsource: kowalski\nhorde: knowledge-compiler\nrun: run-1\ncreated: 2026-10-01\ntags:\n  - kowalski\n---\n\n# Rust"), "{text}");
        let b = save_note(vault.path(), &note("What is: Rust? / async", "again")).unwrap();
        assert_eq!(b, vault.path().join("Kowalski/2026-10-01 What is Rust async (2).md"));
    }

    #[test]
    fn keeps_the_notes_own_front_matter() {
        let vault = tempfile::tempdir().unwrap();
        let body = "---\ntitle: Rust async\ntags:\n  - rust\n---\n\n# Rust async\n";
        let p = save_note(vault.path(), &note("Rust async", body)).unwrap();
        let text = std::fs::read_to_string(p).unwrap();
        assert!(text.starts_with("---\ntitle: Rust async\ntags:\n  - rust\nsource: kowalski\n"), "{text}");
        assert_eq!(text.matches("tags:").count(), 1, "its tags are kept, not doubled");
        assert!(text.ends_with("# Rust async\n"));
    }

    #[test]
    fn an_unclosed_front_matter_block_is_merged_not_doubled() {
        let vault = tempfile::tempdir().unwrap();
        let body = "---\ntitle: Learning Rust\ntags:\n  - rust\n  - learning\n\n# Learning Rust\n\nbody\n";
        let p = save_note(vault.path(), &note("Learning Rust", body)).unwrap();
        let text = std::fs::read_to_string(p).unwrap();
        assert!(text.starts_with("---\ntitle: Learning Rust\ntags:\n  - rust\n  - learning\nsource: kowalski\n"), "{text}");
        assert_eq!(text.matches("---").count(), 2, "one block: {text}");
        assert!(text.contains("---\n\n# Learning Rust\n\nbody"), "{text}");
    }

    #[test]
    fn a_missing_vault_is_an_error() {
        let err = save_note(Path::new("/definitely/not/a/vault"), &note("x", "y")).unwrap_err();
        assert!(err.to_string().contains("not a folder"), "{err}");
    }
}
