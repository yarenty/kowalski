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

fn md_cell(input: &str) -> String {
    input.replace('|', "\\|").replace('\n', " ")
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
    doc.push_str("## Sources Metadata\n\n");
    doc.push_str("| # | Type | Source | Status | Chars | Notes |\n");
    doc.push_str("|---:|---|---|---|---:|---|\n");

    let mut sections = String::new();

    for (idx, asset) in assets.iter().enumerate() {
        match asset {
            InputAsset::Url(url) => {
                let section = match reqwest_blocking::get(url) {
                    Ok(resp) => {
                        let text = resp
                            .text()
                            .unwrap_or_else(|_| "(unable to decode body)".to_string());
                        let clipped = text.chars().take(24000).collect::<String>();
                        doc.push_str(&format!(
                            "| {} | url | {} | ok | {} | fetched |\n",
                            idx + 1,
                            md_cell(url),
                            clipped.chars().count()
                        ));
                        format!(
                            "<!-- source:{}:url:begin -->\n## Source {}: URL\n\n- URL: `{}`\n\n{}\n\n<!-- source:{}:url:end -->\n\n",
                            idx + 1,
                            idx + 1,
                            url,
                            clipped,
                            idx + 1
                        )
                    }
                    Err(e) => {
                        let err = format!("Fetch error: {}", e);
                        doc.push_str(&format!(
                            "| {} | url | {} | error | 0 | {} |\n",
                            idx + 1,
                            md_cell(url),
                            md_cell(&err)
                        ));
                        format!(
                            "<!-- source:{}:url:begin -->\n## Source {}: URL\n\n- URL: `{}`\n\n{}\n\n<!-- source:{}:url:end -->\n\n",
                            idx + 1,
                            idx + 1,
                            url,
                            err,
                            idx + 1
                        )
                    }
                };
                sections.push_str(&section);
            }
            InputAsset::FilePath(path) => {
                let content = fs::read_to_string(path)
                    .unwrap_or_else(|_| "(unable to read file content as text)".to_string());
                let clipped = content.chars().take(24000).collect::<String>();
                doc.push_str(&format!(
                    "| {} | file | {} | ok | {} | local file |\n",
                    idx + 1,
                    path,
                    clipped.chars().count()
                ));
                sections.push_str(&format!(
                    "<!-- source:{}:file:begin -->\n## Source {}: File\n\n- Path: `{}`\n\n{}\n\n<!-- source:{}:file:end -->\n\n",
                    idx + 1,
                    idx + 1,
                    path,
                    clipped,
                    idx + 1
                ));
            }
            InputAsset::Text(text) => {
                let slug = slugify(text);
                doc.push_str(&format!(
                    "| {} | text | {} | ok | {} | direct prompt text |\n",
                    idx + 1,
                    md_cell(if slug.is_empty() { "prompt" } else { &slug }),
                    text.chars().count()
                ));
                sections.push_str(&format!(
                    "<!-- source:{}:text:begin -->\n## Source {}: Text\n\n- Label: `{}`\n\n{}\n\n<!-- source:{}:text:end -->\n\n",
                    idx + 1,
                    idx + 1,
                    if slug.is_empty() { "prompt" } else { &slug },
                    text,
                    idx + 1
                ));
            }
        }
    }
    doc.push('\n');
    doc.push_str("## Source Collection\n\n");
    doc.push_str(&sections);
    fs::write(&out, doc)?;
    Ok(out)
}
