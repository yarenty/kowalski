use chrono::Utc;
use reqwest::blocking as reqwest_blocking;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAsset {
    Url(String),
    FilePath(String),
    Text(String),
}

fn trim_token(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '(' || c == ')')
        .trim_end_matches([',', ';', ':', '.'])
        .to_string()
}

pub fn parse_input_assets(input: &str) -> Vec<InputAsset> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for token in input.split_whitespace() {
        let t = trim_token(token);
        if t.is_empty() {
            continue;
        }
        let asset = if t.starts_with("http://") || t.starts_with("https://") {
            InputAsset::Url(t.clone())
        } else {
            let p = Path::new(&t);
            if p.exists() {
                InputAsset::FilePath(t.clone())
            } else {
                continue;
            }
        };
        let key = format!("{:?}", asset);
        if seen.insert(key) {
            out.push(asset);
        }
    }
    if out.is_empty() {
        let t = input.trim();
        if !t.is_empty() {
            out.push(InputAsset::Text(t.to_string()));
        }
    }
    out
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn ingest_assets_markdown(
    root: &Path,
    source_input: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let assets = parse_input_assets(source_input);
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let out = root
        .join("raw/sources")
        .join(format!("{stamp}-inputs-{}.md", assets.len()));
    let now = Utc::now().to_rfc3339();
    let mut doc = String::new();
    doc.push_str("# Raw Inputs\n\n");
    doc.push_str(&format!(
        "- Inputs: {}\n- Ingested At: {}\n\n",
        assets.len(),
        now
    ));

    for (idx, asset) in assets.iter().enumerate() {
        match asset {
            InputAsset::Url(url) => {
                let section = match reqwest_blocking::get(url) {
                    Ok(resp) => {
                        let text = resp
                            .text()
                            .unwrap_or_else(|_| "(unable to decode body)".to_string());
                        format!(
                            "## Input {} (url): {}\n\n{}\n\n",
                            idx + 1,
                            url,
                            text.chars().take(24000).collect::<String>()
                        )
                    }
                    Err(e) => format!(
                        "## Input {} (url): {}\n\nFetch error: {}\n\n",
                        idx + 1,
                        url,
                        e
                    ),
                };
                doc.push_str(&section);
            }
            InputAsset::FilePath(path) => {
                let content = fs::read_to_string(path)
                    .unwrap_or_else(|_| "(unable to read file content as text)".to_string());
                doc.push_str(&format!(
                    "## Input {} (file): {}\n\n{}\n\n",
                    idx + 1,
                    path,
                    content.chars().take(24000).collect::<String>()
                ));
            }
            InputAsset::Text(text) => {
                let slug = slugify(text);
                doc.push_str(&format!(
                    "## Input {} (text): {}\n\n{}\n\n",
                    idx + 1,
                    if slug.is_empty() { "prompt" } else { &slug },
                    text
                ));
            }
        }
    }
    fs::write(&out, doc)?;
    Ok(out)
}
