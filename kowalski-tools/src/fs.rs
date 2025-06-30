use std::fs;
use std::io::{self, BufRead, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// List files and directories in the given path.
pub fn list_dir<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>, FsError> {
    let entries = fs::read_dir(&path)?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry?;
        result.push(entry.path());
    }
    Ok(result)
}

/// Recursively find files matching a pattern in a directory.
pub fn find_files<P: AsRef<Path>>(dir: P, pattern: &str) -> Result<Vec<PathBuf>, FsError> {
    let mut result = Vec::new();
    find_files_recursive(dir.as_ref(), pattern, &mut result)?;
    Ok(result)
}

fn find_files_recursive(dir: &Path, pattern: &str, result: &mut Vec<PathBuf>) -> Result<(), FsError> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_files_recursive(&path, pattern, result)?;
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains(pattern) {
                    result.push(path);
                }
            }
        }
    }
    Ok(())
}

/// Get the full contents of a file as a String.
pub fn get_file_contents<P: AsRef<Path>>(path: P) -> Result<String, FsError> {
    let contents = fs::read_to_string(&path)?;
    Ok(contents)
}

/// Get the first `num_lines` lines of a file as a Vec<String>.
pub fn get_file_first_lines<P: AsRef<Path>>(path: P, num_lines: usize) -> Result<Vec<String>, FsError> {
    let file = fs::File::open(&path)?;
    let reader = io::BufReader::new(file);
    Ok(reader.lines().take(num_lines).collect::<Result<_, _>>()?)
}

/// Get the last `num_lines` lines of a file as a Vec<String>.
pub fn get_file_last_lines<P: AsRef<Path>>(path: P, num_lines: usize) -> Result<Vec<String>, FsError> {
    let file = fs::File::open(&path)?;
    let mut reader = io::BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.by_ref().lines() {
        lines.push(line?);
    }
    let len = lines.len();
    let start = if num_lines > len { 0 } else { len - num_lines };
    Ok(lines[start..].to_vec())
} 