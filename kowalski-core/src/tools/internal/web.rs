//! In-process **web** helpers: HTTP fetch and a lightweight **HTML → readable Markdown** pass.
//!
//! This is the **default** path when a URL returns HTML (non-GitHub or GitHub HTML fallback).
//! For high-fidelity extraction (readability, paywalls, JS), prefer an **MCP** server or the
//! [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/).

use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;

const FETCH_TIMEOUT_SECS: u64 = 90;

/// Marker for the web helpers module; the agent tools built on it live in `web_tools`.
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

fn flatten_inline_tags(html: &str) -> String {
    let re_tags = Regex::new(r"<[^>]+>").expect("valid regex");
    let t = re_tags.replace_all(html, " ");
    html_entities::decode_html_entities(t.trim())
}

fn decode_href_entities(url: &str) -> String {
    html_entities::decode_html_entities(url.trim())
}

/// Strip scripts/styles and tags; collapse whitespace into Markdown-ish plain text blocks.
/// Preserves hyperlink targets as `[text](url)` before generic tag stripping so URLs are not lost.
/// This is **not** a full Readability clone — it makes HTML **usable** in LLM/source bundles.
pub fn html_body_to_markdown(html: &str) -> String {
    html_to_markdown_at(html, None)
}

/// The page's own content: the `<main>` element (else a lone `<article>`) when there is one,
/// without navigation, header, footer, menus and other page chrome.
fn main_content(html: &str) -> std::borrow::Cow<'_, str> {
    // an unclosed <main> (a body cut at the fetch cap) runs to the end of what arrived
    let re_main = Regex::new(r"(?is)<main\b[^>]*>(.*?)(?:</main\s*>|\z)").expect("valid regex");
    let re_article = Regex::new(r"(?is)<article\b[^>]*>(.*?)</article\s*>").expect("valid regex");
    // one <article> is the page's content; many are rows of a list, so keep the page
    let single_article = || {
        let mut all = re_article.captures_iter(html);
        match (all.next(), all.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        }
    };
    let picked = re_main
        .captures(html)
        .or_else(single_article)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .filter(|m| m.len() > 200);
    let mut out = picked.unwrap_or(html).to_string();
    for tag in ["nav", "header", "footer", "aside", "svg", "noscript", "template", "form", "details", "select", "dialog"] {
        let re = Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}\s*>")).expect("valid regex");
        out = re.replace_all(&out, " ").into_owned();
    }
    std::borrow::Cow::Owned(out)
}

/// [`html_body_to_markdown`] for a page fetched from `base`: relative links become absolute.
pub fn html_to_markdown_at(html: &str, base: Option<&reqwest::Url>) -> String {
    let title = Regex::new(r"(?is)<title[^>]*>(.*?)</title>")
        .expect("valid regex")
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| flatten_inline_tags(m.as_str()))
        .filter(|t| !t.is_empty());
    let content = main_content(html);
    let html = content.as_ref();
    let re_script = Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("valid regex");
    let re_style = Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("valid regex");
    let re_anchor = Regex::new(
        r#"(?is)<a\s[^>]*?\bhref\s*=\s*(?:"(?P<dq>[^"]*)"|'(?P<sq>[^']*)')[^>]*>(?P<inner>.*?)</a>"#,
    )
    .expect("valid regex");
    let re_tags = Regex::new(r"<[^>]+>").expect("valid regex");
    let re_ws = Regex::new(r"[ \t\r\f\v]+").expect("valid regex");
    let re_nl = Regex::new(r"\n{3,}").expect("valid regex");

    let s = re_script.replace_all(html, "");
    let s = re_style.replace_all(&s, "");
    let s = re_anchor.replace_all(&s, |caps: &regex::Captures| {
        let url = caps
            .name("dq")
            .or_else(|| caps.name("sq"))
            .map(|m| decode_href_entities(m.as_str()))
            .unwrap_or_default();
        let url = match base {
            Some(b) if !url.is_empty() && !url.starts_with('#') && !url.contains("://") => {
                b.join(&url).map(|u| u.to_string()).unwrap_or(url)
            }
            _ => url,
        };
        let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
        let text = flatten_inline_tags(inner);
        if url.is_empty() {
            return text;
        }
        let low = url.to_ascii_lowercase();
        if low.starts_with("javascript:") || low.starts_with("data:") {
            return text;
        }
        if text.is_empty() {
            return format!("<{url}>");
        }
        if url.contains(' ') && !url.starts_with('<') {
            return format!("[{text}](<{url}>)");
        }
        if url.contains(')') {
            return format!("[{text}](<{url}>)");
        }
        format!("[{text}]({url})")
    });
    let s = re_tags.replace_all(&s, " ");
    let s: String = html_entities::decode_html_entities(s.as_ref());
    let s = re_ws.replace_all(&s, " ");
    let lines: Vec<&str> = s.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let body = lines.join("\n\n");
    let body = re_nl.replace_all(&body, "\n\n");
    let heading = title.map(|t| format!("# {t}\n\n")).unwrap_or_default();
    format!(
        "<!-- converted from HTML (internal web tool; heuristic strip) -->\n\n{heading}{}\n",
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

/// A GET that waits once and tries again when the site answers "429 Too Many Requests": the
/// `Retry-After` it asks for, at most 10 s, else 5 s. Shared by every ingest fetch.
pub(crate) fn get_politely(client: &Client, url: &str) -> Result<reqwest::blocking::Response, String> {
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if resp.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Ok(resp);
    }
    std::thread::sleep(retry_wait(resp.headers()));
    client.get(url).send().map_err(|e| e.to_string())
}

/// Longest pause before the one retry of a rate-limited request.
const MAX_RETRY_WAIT_SECS: u64 = 65;

/// How long a server that answered 429 asks us to wait: `Retry-After`, else the seconds until
/// its rate-limit window resets (`x-ratelimit-reset`, as Reddit sends), else five seconds.
fn retry_wait(headers: &reqwest::header::HeaderMap) -> Duration {
    let seconds = |name: &str| {
        headers.get(name).and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<f64>().ok())
    };
    let wait = seconds(reqwest::header::RETRY_AFTER.as_str())
        .or_else(|| seconds("x-ratelimit-reset").map(|s| s + 1.0))
        .unwrap_or(5.0);
    Duration::from_secs((wait.max(0.0).ceil() as u64).min(MAX_RETRY_WAIT_SECS))
}

/// A response body as text, or the HTTP status as the error. The whole page (up to the shared
/// cap) is read before it is cleaned up: a busy page such as GitHub trending has half a
/// megabyte of menus before its list, so cutting the raw HTML early lost the content. The
/// steps' own context limits keep what a model sees small.
pub(crate) fn read_body_text(resp: reqwest::blocking::Response) -> Result<String, String> {
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {} {}",
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or("")
        ));
    }
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    let bytes = &bytes[..bytes.len().min(super::web_tools::MAX_BODY_BYTES)];
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// Plain HTTP GET body as UTF-8 string (no GitHub resolution).
pub fn fetch_http_body(url: &str) -> Result<String, String> {
    let client = http_client()?;
    read_body_text(get_politely(&client, url)?)
}

/// How much of each item of an RSS or Atom feed is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedItems {
    /// A list to choose from: one line per item, its text cut to about two sentences.
    #[default]
    Summaries,
    /// Pages to read: each item under its own heading with its whole text (a discussion
    /// thread's feed is the post followed by its replies).
    Whole,
}

/// Fetch URL as readable text (see [`page_to_markdown`]).
pub fn fetch_url_as_markdown(url: &str) -> Result<String, String> {
    fetch_url_as_markdown_with(url, FeedItems::Summaries)
}

/// [`fetch_url_as_markdown`] with a choice of how much of a feed's items is kept.
pub fn fetch_url_as_markdown_with(url: &str, feed_items: FeedItems) -> Result<String, String> {
    let text = fetch_http_body(url)?;
    Ok(page_to_markdown_with(&text, reqwest::Url::parse(url).ok().as_ref(), feed_items))
}

/// A fetched body as readable Markdown: an RSS or Atom feed becomes a list of its items, HTML
/// becomes its main content, anything else is kept as it is.
pub fn page_to_markdown(text: &str, base: Option<&reqwest::Url>) -> String {
    page_to_markdown_with(text, base, FeedItems::Summaries)
}

/// [`page_to_markdown`] with a choice of how much of a feed's items is kept.
pub fn page_to_markdown_with(text: &str, base: Option<&reqwest::Url>, feed_items: FeedItems) -> String {
    if let Some(md) = feed_to_markdown(text, feed_items) {
        return md;
    }
    if looks_like_html(text) { html_to_markdown_at(text, base) } else { text.to_string() }
}

/// An RSS or Atom feed as Markdown; `None` when `text` is not a feed. [`FeedItems::Summaries`]
/// gives `- [title](link) — summary` lines (summary cut to about two sentences),
/// [`FeedItems::Whole`] gives a `## [title](link)` heading and the whole text per item.
fn feed_to_markdown(text: &str, feed_items: FeedItems) -> Option<String> {
    let head = &text[..text.len().min(2048)];
    if !(head.contains("<rss") || head.contains("<feed") || head.contains("<rdf:RDF")) {
        return None;
    }
    let re_item = Regex::new(r"(?is)<(item|entry)\b[^>]*>(.*?)</(?:item|entry)\s*>").expect("valid regex");
    let re_title = Regex::new(r"(?is)<title\b[^>]*>(.*?)</title\s*>").expect("valid regex");
    let re_link_text = Regex::new(r"(?is)<link\b[^>]*>(.*?)</link\s*>").expect("valid regex");
    let re_link_href = Regex::new(r#"(?is)<link\b[^>]*\bhref\s*=\s*"([^"]+)""#).expect("valid regex");
    let re_summary = Regex::new(r"(?is)<(description|summary|content)\b[^>]*>(.*?)</(?:description|summary|content)\s*>").expect("valid regex");
    let re_tags = Regex::new(r"<[^>]+>").expect("valid regex");
    let clean = |raw: &str| {
        let raw = raw.trim().trim_start_matches("<![CDATA[").trim_end_matches("]]>");
        // escaped HTML inside a feed: decode, drop the tags, decode what is left
        let decoded = html_entities::decode_html_entities(raw);
        let plain = re_tags.replace_all(&decoded, " ");
        let plain = html_entities::decode_html_entities(&plain);
        plain.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let feed_title = re_title.captures(&text[..text.find("<item").or_else(|| text.find("<entry")).unwrap_or(text.len())])
        .map(|c| clean(&c[1]))
        .filter(|t| !t.is_empty());
    let mut out = String::from("<!-- converted from an RSS/Atom feed (internal web tool) -->\n\n");
    if let Some(t) = feed_title {
        out.push_str(&format!("# {t}\n\n"));
    }
    let mut items = 0;
    for c in re_item.captures_iter(text) {
        let body = &c[2];
        let title = re_title.captures(body).map(|m| clean(&m[1])).unwrap_or_default();
        let link = re_link_text
            .captures(body)
            .map(|m| clean(&m[1]))
            .filter(|l| !l.is_empty())
            .or_else(|| re_link_href.captures(body).map(|m| m[1].to_string()))
            .unwrap_or_default();
        let mut summary = re_summary.captures(body).map(|m| clean(&m[2])).unwrap_or_default();
        if feed_items == FeedItems::Summaries && summary.chars().count() > 240 {
            let cut: String = summary.chars().take(240).collect();
            summary = format!("{}…", cut.rsplit_once(' ').map_or(cut.as_str(), |(a, _)| a));
        }
        if title.is_empty() && link.is_empty() {
            continue;
        }
        let head = if link.is_empty() { title } else { format!("[{title}]({link})") };
        out.push_str(&match (feed_items, summary.is_empty()) {
            (FeedItems::Whole, true) => format!("## {head}\n\n"),
            (FeedItems::Whole, false) => format!("## {head}\n\n{summary}\n\n"),
            (FeedItems::Summaries, true) => format!("- {head}\n"),
            (FeedItems::Summaries, false) => format!("- {head} — {summary}\n"),
        });
        items += 1;
    }
    (items > 0).then_some(out)
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

    #[test]
    fn keeps_main_content_and_resolves_relative_links() {
        let html = r#"<html><head><title>Trending Rust</title></head><body>
            <header><a href="/login">Sign in</a></header><nav>Platform Copilot Actions</nav>
            <main><h2><a href="/tokio-rs/tokio">tokio-rs / tokio</a></h2>
            <p>A runtime for writing reliable asynchronous applications with Rust. Stars today: 120.
            More text so the main element is clearly the page's own content, not a stub.</p></main>
            <footer>Terms Privacy</footer></body></html>"#;
        let base = reqwest::Url::parse("https://github.com/trending/rust").unwrap();
        let md = html_to_markdown_at(html, Some(&base));
        assert!(md.contains("# Trending Rust"), "{md}");
        assert!(md.contains("[tokio-rs / tokio](https://github.com/tokio-rs/tokio)"), "{md}");
        assert!(!md.contains("Sign in") && !md.contains("Copilot") && !md.contains("Privacy"), "{md}");
    }

    #[test]
    fn a_main_cut_off_by_the_fetch_cap_still_counts() {
        let html = format!(
            "<html><body><nav>Menu Menu</nav><main><p>{}</p><a href=\"/a/b\">a / b</a>",
            "Repository description text. ".repeat(20)
        );
        let md = html_to_markdown_at(&html, Some(&reqwest::Url::parse("https://github.com/x").unwrap()));
        assert!(md.contains("[a / b](https://github.com/a/b)") && !md.contains("Menu"), "{md}");
    }

    #[test]
    fn preserves_anchors_as_markdown_links() {
        let html = r#"<html><body><p>See <a href="https://example.com/path?q=1&amp;r=2">Example</a> now.</p></body></html>"#;
        let md = html_body_to_markdown(html);
        assert!(md.contains("[Example](https://example.com/path?q=1&r=2)"));
    }

    #[test]
    fn feeds_become_item_lists() {
        let rss = r#"<?xml version="1.0"?><rss version="2.0"><channel><title>cs.AI updates on arXiv.org</title>
<item><title>A Paper &amp; Its Lessons</title><link>https://arxiv.org/abs/2609.1</link>
<description>arXiv:2609.1 Abstract: We study &lt;b&gt;agents&lt;/b&gt;. They are useful.</description></item>
<item><title>Second</title><link>https://arxiv.org/abs/2609.2</link><description>Short.</description></item>
</channel></rss>"#;
        let md = page_to_markdown(rss, None);
        assert!(md.contains("# cs.AI updates on arXiv.org"), "{md}");
        assert!(md.contains("- [A Paper & Its Lessons](https://arxiv.org/abs/2609.1) — arXiv:2609.1 Abstract: We study agents . They are useful."), "{md}");
        assert!(md.contains("- [Second](https://arxiv.org/abs/2609.2) — Short."), "{md}");
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Blog</title>
<entry><title>Post</title><link href="https://example.com/post"/><summary>Hello.</summary></entry></feed>"#;
        assert!(page_to_markdown(atom, None).contains("- [Post](https://example.com/post) — Hello."));
        assert!(!page_to_markdown("<html><body><p>Not a feed</p></body></html>", None).contains("RSS"));
    }

    #[test]
    fn a_rate_limited_request_waits_as_long_as_the_server_asks() {
        use reqwest::header::{HeaderMap, HeaderValue};
        let headers = |pairs: &[(&'static str, &'static str)]| {
            let mut map = HeaderMap::new();
            for (name, value) in pairs {
                map.insert(*name, HeaderValue::from_static(value));
            }
            map
        };
        assert_eq!(retry_wait(&headers(&[])), Duration::from_secs(5));
        assert_eq!(retry_wait(&headers(&[("retry-after", "3")])), Duration::from_secs(3));
        assert_eq!(retry_wait(&headers(&[("x-ratelimit-reset", "46")])), Duration::from_secs(47));
        assert_eq!(
            retry_wait(&headers(&[("retry-after", "2"), ("x-ratelimit-reset", "46")])),
            Duration::from_secs(2)
        );
        assert_eq!(retry_wait(&headers(&[("retry-after", "3600")])), Duration::from_secs(65));
    }

    #[test]
    fn a_feed_read_whole_keeps_every_item_in_full() {
        let long = "word ".repeat(200);
        let atom = format!(
            r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Thread</title>
<entry><title>The post</title><link href="https://example.com/t/1"/><content type="html">&lt;p&gt;{long}&lt;/p&gt;</content></entry>
<entry><title>A reply</title><link href="https://example.com/t/1/c"/><content>Short reply.</content></entry></feed>"#
        );
        let listed = page_to_markdown(&atom, None);
        assert!(listed.contains("- [The post](https://example.com/t/1) — word"));
        assert!(listed.contains('…'), "a listing cuts long items");

        let whole = page_to_markdown_with(&atom, None, FeedItems::Whole);
        assert!(whole.contains("## [The post](https://example.com/t/1)\n\nword word"));
        assert!(whole.contains(long.trim()), "the whole text is kept");
        assert!(!whole.contains('…'));
        assert!(whole.contains("## [A reply](https://example.com/t/1/c)\n\nShort reply."));
    }

}
