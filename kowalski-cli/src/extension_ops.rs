//! Generic extension runners for `kowalski-cli`.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn extension_binary_name(name: &str) -> String {
    format!("kowalski-ext-{}", name)
}

fn local_extension_runner(name: &str) -> PathBuf {
    PathBuf::from(".kowalski")
        .join("extensions")
        .join(name)
        .join("run")
}

fn has_execute_bit(meta: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return meta.permissions().mode() & 0o111 != 0;
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        true
    }
}

pub fn run_extension(name: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let bin = extension_binary_name(name);
    let status = Command::new(&bin).args(args).status();
    match status {
        Ok(code) => {
            if !code.success() {
                return Err(format!("Extension `{}` failed with status {}", name, code).into());
            }
            return Ok(());
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Box::new(e)),
    }

    let local = local_extension_runner(name);
    if local.exists() {
        let meta = fs::metadata(&local)?;
        if !meta.is_file() || !has_execute_bit(&meta) {
            return Err(format!(
                "Local extension runner exists but is not executable: {}",
                local.display()
            )
            .into());
        }
        let status = Command::new(&local).args(args).status()?;
        if !status.success() {
            return Err(format!("Local extension `{}` failed with status {}", name, status).into());
        }
        return Ok(());
    }

    Err(format!(
        "Extension `{}` not found.\nExpected either `{}` in PATH or `{}` as an executable.",
        name,
        bin,
        local.display()
    )
    .into())
}

pub fn list_extensions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut out = BTreeSet::new();
    let prefix = "kowalski-ext-";

    if let Some(path_var) = env::var_os("PATH") {
        for dir in env::split_paths(&path_var) {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str()
                        && let Some(ext_name) = name.strip_prefix(prefix)
                        && !ext_name.is_empty()
                    {
                        out.insert(ext_name.to_string());
                    }
                }
            }
        }
    }

    let local_root = Path::new(".kowalski/extensions");
    if local_root.exists() {
        for entry in fs::read_dir(local_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && let Some(name) = path.file_name().and_then(|x| x.to_str()) {
                out.insert(name.to_string());
            }
        }
    }

    Ok(out.into_iter().collect())
}
