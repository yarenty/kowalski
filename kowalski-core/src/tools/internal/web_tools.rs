//! Web tools for agents: `web_fetch` (a page as readable Markdown) and `web_search` (a search
//! API the operator configured). Both run in-process; for browsing that needs JavaScript or a
//! login, attach an MCP server instead.
//!
//! `web_fetch` only talks to public addresses: a model must not be steerable into the
//! operator's router, local services or a cloud metadata endpoint. Every redirect hop is
//! checked the same way.

use crate::error::KowalskiError;
use crate::tools::internal::web::{html_body_to_markdown, looks_like_html};
use crate::tools::{ParameterType, Tool, ToolInput, ToolOutput, ToolParameter};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::net::IpAddr;
use std::time::Duration;

const FETCH_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 5;
const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;
const DEFAULT_MAX_CHARS: usize = 20_000;

fn param(name: &str, description: &str, required: bool, ty: ParameterType, default: Option<&str>) -> ToolParameter {
    ToolParameter {
        name: name.into(),
        description: description.into(),
        required,
        default_value: default.map(str::to_string),
        parameter_type: ty,
    }
}

fn arg_str<'a>(input: &'a ToolInput, key: &str) -> Option<&'a str> {
    input.parameters.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

fn arg_usize(input: &ToolInput, key: &str) -> Option<usize> {
    let v = input.parameters.get(key)?;
    v.as_u64().map(|n| n as usize).or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// Whether an address is on the public internet (not loopback, private, link-local, carrier-grade
/// NAT, documentation, multicast or unspecified).
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_documentation()
                || o[0] == 0
                || (o[0] == 100 && (64..=127).contains(&o[1])))
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(v4));
            }
            let seg = v6.segments();
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (seg[0] & 0xfe00) == 0xfc00 // unique local
                || (seg[0] & 0xffc0) == 0xfe80 // link local
                || seg[0] == 0x2001 && seg[1] == 0x0db8) // documentation
        }
    }
}

async fn check_public(url: &reqwest::Url) -> Result<(), String> {
    match url.scheme() {
        "http" | "https" => {}
        s => return Err(format!("only http and https URLs can be fetched, not `{s}`")),
    }
    let host = url.host_str().ok_or("URL has no host")?;
    let port = url.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host.trim_matches(|c| c == '[' || c == ']'), port))
        .await
        .map_err(|e| format!("cannot resolve {host}: {e}"))?;
    let mut any = false;
    for a in addrs {
        any = true;
        if !is_public_ip(a.ip()) {
            return Err(format!("{host} resolves to a private or local address ({}); web_fetch only reaches the public internet", a.ip()));
        }
    }
    if !any {
        return Err(format!("{host} did not resolve"));
    }
    Ok(())
}

/// Fetch a public URL (manual redirects, each hop checked) and return the body as text,
/// HTML converted to readable Markdown.
pub async fn fetch_public_as_markdown(url: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("Kowalski/", env!("CARGO_PKG_VERSION"), " (+https://github.com/yarenty/kowalski)"))
        .build()
        .map_err(|e| e.to_string())?;
    let mut current = reqwest::Url::parse(url).map_err(|e| format!("not a URL: {e}"))?;
    for _ in 0..=MAX_REDIRECTS {
        check_public(&current).await?;
        let res = client.get(current.clone()).send().await.map_err(|e| e.to_string())?;
        if res.status().is_redirection() {
            let loc = res
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or("redirect without a Location header")?;
            current = current.join(loc).map_err(|e| format!("bad redirect: {e}"))?;
            continue;
        }
        if !res.status().is_success() {
            return Err(format!("HTTP {}", res.status()));
        }
        let final_url = current.to_string();
        let bytes = res.bytes().await.map_err(|e| e.to_string())?;
        let bytes = &bytes[..bytes.len().min(MAX_BODY_BYTES)];
        let text = String::from_utf8_lossy(bytes).into_owned();
        let text = if looks_like_html(&text) { html_body_to_markdown(&text) } else { text };
        return Ok((final_url, text));
    }
    Err(format!("more than {MAX_REDIRECTS} redirects"))
}

/// `web_fetch`: one public web page as readable Markdown.
#[derive(Debug, Clone, Default)]
pub struct WebFetchTool;

#[async_trait]
impl Tool for WebFetchTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let url = arg_str(&input, "url").ok_or_else(|| KowalskiError::ToolInvalidInput("missing `url`".into()))?;
        let max = arg_usize(&input, "max_chars").unwrap_or(DEFAULT_MAX_CHARS).clamp(500, 200_000);
        let (final_url, text) = fetch_public_as_markdown(url).await.map_err(KowalskiError::ToolExecution)?;
        let truncated = text.chars().count() > max;
        let content: String = text.chars().take(max).collect();
        Ok(ToolOutput::new(
            json!({ "url": final_url, "content": content, "truncated": truncated }),
            None,
        ))
    }

    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch one public web page and return its readable text (Markdown, links kept). Use for a URL the user gave or one found with web_search. Public internet only."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            param("url", "http(s) URL of the page", true, ParameterType::String, None),
            param("max_chars", "Longest text to return (default 20000)", false, ParameterType::Number, Some("20000")),
        ]
    }
}

/// Which search backend `web_search` uses (`[search]` in the config).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchBackend {
    /// Brave Search API (free tier available); key from config or `BRAVE_API_KEY`.
    Brave { api_key: String },
    /// A SearXNG instance with the JSON format enabled.
    Searxng { url: String },
    /// Staan (European web index, staan.ai); key from config or `STAAN_API_KEY`, optional
    /// `market` such as `en-us`, `de-de`, `fr-fr`.
    Staan { api_key: String, market: Option<String> },
}

impl SearchBackend {
    /// From the config's `[search]` table: `provider = "brave"` (+ `api_key`, or `BRAVE_API_KEY`)
    /// or `provider = "searxng"` + `url`. `None` when not configured: the tool is not offered.
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        let s = config.additional.get("search")?;
        let get = |k: &str| s.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
        match get("provider")?.to_ascii_lowercase().as_str() {
            "brave" => {
                let api_key = get("api_key").or_else(|| std::env::var("BRAVE_API_KEY").ok().filter(|k| !k.is_empty()))?;
                Some(Self::Brave { api_key })
            }
            "searxng" => Some(Self::Searxng { url: get("url")?.trim_end_matches('/').to_string() }),
            "staan" => {
                let api_key = get("api_key").or_else(|| std::env::var("STAAN_API_KEY").ok().filter(|k| !k.is_empty()))?;
                Some(Self::Staan { api_key, market: get("market") })
            }
            other => {
                log::warn!("[search] provider `{other}` is not supported (brave, staan, searxng); web_search disabled");
                None
            }
        }
    }
}

/// `web_search`: top results (title, URL, snippet) for a query.
#[derive(Debug, Clone)]
pub struct WebSearchTool {
    backend: SearchBackend,
    http: reqwest::Client,
}

impl WebSearchTool {
    /// A search tool over `backend`.
    pub fn new(backend: SearchBackend) -> Self {
        Self {
            backend,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent(concat!("Kowalski/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn search(&self, query: &str, count: usize) -> Result<Vec<Value>, String> {
        let hit = |title: Option<&str>, url: Option<&str>, snippet: Option<&str>| {
            json!({ "title": title.unwrap_or(""), "url": url.unwrap_or(""), "snippet": snippet.unwrap_or("") })
        };
        match &self.backend {
            SearchBackend::Brave { api_key } => {
                let res = self
                    .http
                    .get("https://api.search.brave.com/res/v1/web/search")
                    .query(&[("q", query), ("count", &count.to_string())])
                    .header("X-Subscription-Token", api_key)
                    .header("Accept", "application/json")
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("Brave Search answered {}", res.status()));
                }
                let v: Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(v["web"]["results"]
                    .as_array()
                    .map(|a| a.iter().take(count).map(|r| hit(r["title"].as_str(), r["url"].as_str(), r["description"].as_str())).collect())
                    .unwrap_or_default())
            }
            SearchBackend::Staan { api_key, market } => {
                let mut req = self
                    .http
                    .get("https://api.staan.ai/v2/search/web")
                    .query(&[("q", query)])
                    .bearer_auth(api_key);
                if let Some(m) = market {
                    req = req.query(&[("market", m.as_str())]);
                }
                let res = req.send().await.map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("Staan answered {}", res.status()));
                }
                let v: Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(v["web"]["results"]
                    .as_array()
                    .map(|a| a.iter().take(count).map(|r| hit(r["title"].as_str(), r["url"].as_str(), r["snippet"].as_str())).collect())
                    .unwrap_or_default())
            }
            SearchBackend::Searxng { url } => {
                let res = self
                    .http
                    .get(format!("{url}/search"))
                    .query(&[("q", query), ("format", "json")])
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("SearXNG answered {} (is the json format enabled?)", res.status()));
                }
                let v: Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(v["results"]
                    .as_array()
                    .map(|a| a.iter().take(count).map(|r| hit(r["title"].as_str(), r["url"].as_str(), r["content"].as_str())).collect())
                    .unwrap_or_default())
            }
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    async fn execute(&mut self, input: ToolInput) -> Result<ToolOutput, KowalskiError> {
        let query = arg_str(&input, "query").ok_or_else(|| KowalskiError::ToolInvalidInput("missing `query`".into()))?;
        let count = arg_usize(&input, "count").unwrap_or(5).clamp(1, 10);
        let results = self.search(query, count).await.map_err(KowalskiError::ToolExecution)?;
        Ok(ToolOutput::new(json!({ "query": query, "results": results }), None))
    }

    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web and return the top results (title, URL, snippet). Follow up with web_fetch to read a result."
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            param("query", "What to search for", true, ParameterType::String, None),
            param("count", "Number of results, 1-10 (default 5)", false, ParameterType::Number, Some("5")),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_and_special_addresses_are_refused() {
        for ip in ["127.0.0.1", "10.1.2.3", "192.168.0.1", "172.16.5.4", "169.254.169.254", "100.64.0.1", "0.0.0.0", "::1", "fe80::1", "fd00::1", "::ffff:127.0.0.1"] {
            assert!(!is_public_ip(ip.parse().unwrap()), "{ip} must be refused");
        }
        for ip in ["1.1.1.1", "93.184.216.34", "2606:4700:4700::1111"] {
            assert!(is_public_ip(ip.parse().unwrap()), "{ip} is public");
        }
    }

    #[tokio::test]
    async fn fetch_refuses_local_targets_and_other_schemes() {
        let err = fetch_public_as_markdown("http://127.0.0.1:3456/api/health").await.unwrap_err();
        assert!(err.contains("private or local"), "{err}");
        let err = fetch_public_as_markdown("http://localhost/").await.unwrap_err();
        assert!(err.contains("private or local"), "{err}");
        let err = fetch_public_as_markdown("file:///etc/passwd").await.unwrap_err();
        assert!(err.contains("only http and https"), "{err}");
    }

    #[tokio::test]
    async fn live_fetch_when_asked() {
        if std::env::var("KOWALSKI_LIVE_WEB").is_err() {
            return; // network check, opt in with KOWALSKI_LIVE_WEB=1
        }
        let (url, text) = fetch_public_as_markdown("http://example.com/").await.unwrap();
        assert!(url.starts_with("http"), "{url}");
        assert!(text.contains("Example Domain"), "{text}");
        assert!(!text.contains("<p>"), "HTML became text");
    }

    #[tokio::test]
    async fn agents_get_web_fetch_always_and_web_search_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = crate::config::Config::default();
        c.memory.episodic_path = dir.path().join("episodic").display().to_string();
        let names = |tools: Vec<(String, String)>| tools.into_iter().map(|(n, _)| n).collect::<Vec<_>>();
        let plain = crate::template::agent::TemplateAgent::new(c.clone()).await.unwrap();
        let tools = names(plain.list_tools().await);
        assert!(tools.contains(&"web_fetch".to_string()) && !tools.contains(&"web_search".to_string()), "{tools:?}");
        c.additional.insert("search".into(), json!({ "provider": "searxng", "url": "https://search.example" }));
        let searching = crate::template::agent::TemplateAgent::new(c).await.unwrap();
        let tools = names(searching.list_tools().await);
        assert!(tools.contains(&"web_search".to_string()), "{tools:?}");
    }

    #[test]
    fn search_backend_from_config() {
        let mut c = crate::config::Config::default();
        assert_eq!(SearchBackend::from_config(&c), None);
        c.additional.insert("search".into(), json!({ "provider": "searxng", "url": "https://search.example/" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Searxng { url: "https://search.example".into() }));
        c.additional.insert("search".into(), json!({ "provider": "brave", "api_key": "k" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Brave { api_key: "k".into() }));
        c.additional.insert("search".into(), json!({ "provider": "staan", "api_key": "s", "market": "de-de" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Staan { api_key: "s".into(), market: Some("de-de".into()) }));
        c.additional.insert("search".into(), json!({ "provider": "bing", "api_key": "k" }));
        assert_eq!(SearchBackend::from_config(&c), None);
    }
}
