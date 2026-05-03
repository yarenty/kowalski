//! In-process **web** helpers: HTTP fetch and a lightweight **HTML → readable Markdown** pass.
//!
//! This is the **default** path when a URL returns HTML (non-GitHub or GitHub HTML fallback).
//! For high-fidelity extraction (readability, paywalls, JS), prefer an **MCP** server or the
//! [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/).

use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;

const FETCH_TIMEOUT_SECS: u64 = 90;

/// Marker for future `Tool` registration (`internal_web_fetch`, etc.).
#[derive(Debug, Clone, Copy, Default)]
pub struct WebInternalModule;

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .user_agent(concat!("Kowalski/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

/// True if the payload is likely HTML (browser page, error HTML, etc.).
pub fn looks_like_html(s: &str) -> bool {
    let t = s.trim_start();
    t.starts_with("<!DOCTYPE")
        || t.starts_with("<!doctype")
        || t.starts_with("<html")
        || t.starts_with("<HTML")
        || (t.contains('<') && t.contains('>') && t[..t.len().min(512)].contains("</"))
}

/// Strip scripts/styles and tags; collapse whitespace into Markdown-ish plain text blocks.
/// This is **not** a full Readability clone — it makes HTML **usable** in LLM/source bundles.
pub fn html_body_to_markdown(html: &str) -> String {
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("valid regex");
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("valid regex");
    let re_tags = Regex::new(r"<[^>]+>").expect("valid regex");
    let re_ws = Regex::new(r"[ \t\r\f\v]+").expect("valid regex");
    let re_nl = Regex::new(r"\n{3,}").expect("valid regex");

    let s = re_script.replace_all(html, "");
    let s = re_style.replace_all(&s, "");
    let s = re_tags.replace_all(&s, " ");
    let s: String = html_entities::decode_html_entities(s.as_ref());
    let s = re_ws.replace_all(&s, " ");
    let lines: Vec<&str> = s.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let body = lines.join("\n\n");
    let body = re_nl.replace_all(&body, "\n\n");
    format!(
        "<!-- converted from HTML (internal web tool; heuristic strip) -->\n\n{}\n",
        body.trim()
    )
}

// Minimal entity decode without pulling `html-escape` for a few cases
mod html_entities {
    pub fn decode_html_entities(s: &str) -> String {
        s.replace("&nbsp;", " ")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
    }
}

/// Plain HTTP GET body as UTF-8 string (no GitHub resolution).
pub fn fetch_http_body(url: &str) -> Result<String, String> {
    let client = http_client()?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {} {}",
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or("")
        ));
    }
    resp.text().map_err(|e| e.to_string())
}

/// Fetch URL; if the body looks like HTML, convert to readable Markdown text.
pub fn fetch_url_as_markdown(url: &str) -> Result<String, String> {
    let text = fetch_http_body(url)?;
    if looks_like_html(&text) {
        Ok(html_body_to_markdown(&text))
    } else {
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_html() {
        assert!(looks_like_html("<html><body>Hi</body></html>"));
        assert!(!looks_like_html("# Just markdown"));
    }

    #[test]
    fn strips_script_and_tags() {
        let html = r#"<html><head><script>evil()</script><style>.x{}</style></head>
            <body><h1>Title</h1><p>Hello <b>world</b></p></body></html>"#;
        let md = html_body_to_markdown(html);
        assert!(!md.contains("evil"));
        assert!(!md.contains("<script"));
        assert!(md.contains("Title"));
        assert!(md.contains("world"));
    }
}
