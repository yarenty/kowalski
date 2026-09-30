//! Web tools for agents: `web_fetch` (a page as readable Markdown) and `web_search` (a search
//! API the operator configured). Both run in-process; for browsing that needs JavaScript or a
//! login, attach an MCP server instead.
//!
//! `web_fetch` only talks to public addresses: a model must not be steerable into the
//! operator's router, local services or a cloud metadata endpoint. Every redirect hop is
//! checked the same way.

use crate::error::KowalskiError;
use crate::tools::internal::web::{html_to_markdown_at, looks_like_html};
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
        let text = if looks_like_html(&text) { html_to_markdown_at(&text, Some(&current)) } else { text };
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
    /// DuckDuckGo's plain-HTML results page: no key, the default. Unofficial (DuckDuckGo has no
    /// web search API), so it may be rate-limited; a keyed provider is the steadier choice.
    DuckDuckGo,
    /// Brave Search API (free tier available); key from config or `BRAVE_API_KEY`.
    Brave { api_key: String },
    /// A SearXNG instance with the JSON format enabled.
    Searxng { url: String },
    /// Staan (European web index, staan.ai); key from config or `STAAN_API_KEY`, optional
    /// `market` such as `en-us`, `de-de`, `fr-fr`.
    Staan { api_key: String, market: Option<String> },
    /// Tavily (search built for agents); key from config or `TAVILY_API_KEY`.
    Tavily { api_key: String },
    /// Google results through Serper (serper.dev); key from config or `SERPER_API_KEY`.
    Serper { api_key: String },
    /// Google Programmable Search: key (`GOOGLE_API_KEY`) plus a search engine id `cx`
    /// (`GOOGLE_CSE_ID`).
    Google { api_key: String, cx: String },
}

/// Providers `[search] provider` accepts, for messages and the Setup screen.
pub const SEARCH_PROVIDERS: &[&str] = &["duckduckgo", "brave", "staan", "tavily", "serper", "google", "searxng", "off"];

impl SearchBackend {
    /// From the config's `[search]` table (`provider`, `api_key`, and `url` for SearXNG, `cx`
    /// for Google, `market` for Staan). No table means DuckDuckGo; `provider = "off"` turns
    /// search off (`None`: the tool is not offered). A keyed provider without its key falls back
    /// to DuckDuckGo, with a warning, rather than leaving the agents without search.
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        let Some(s) = config.additional.get("search") else {
            return Some(Self::DuckDuckGo);
        };
        let get = |k: &str| s.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
        let env = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let key = |var: &str| get("api_key").or_else(|| env(var));
        let provider = get("provider").unwrap_or_else(|| "duckduckgo".into()).to_ascii_lowercase();
        let keyed = |name: &str, backend: Option<Self>| {
            backend.or_else(|| {
                log::warn!("[search] provider `{name}` has no API key; using DuckDuckGo");
                Some(Self::DuckDuckGo)
            })
        };
        match provider.as_str() {
            "off" | "none" => None,
            "duckduckgo" | "ddg" => Some(Self::DuckDuckGo),
            "brave" => keyed("brave", key("BRAVE_API_KEY").map(|api_key| Self::Brave { api_key })),
            "staan" => keyed("staan", key("STAAN_API_KEY").map(|api_key| Self::Staan { api_key, market: get("market") })),
            "tavily" => keyed("tavily", key("TAVILY_API_KEY").map(|api_key| Self::Tavily { api_key })),
            "serper" => keyed("serper", key("SERPER_API_KEY").map(|api_key| Self::Serper { api_key })),
            "google" => keyed(
                "google",
                key("GOOGLE_API_KEY")
                    .zip(get("cx").or_else(|| env("GOOGLE_CSE_ID")))
                    .map(|(api_key, cx)| Self::Google { api_key, cx }),
            ),
            "searxng" => match get("url") {
                Some(url) => Some(Self::Searxng { url: url.trim_end_matches('/').to_string() }),
                None => {
                    log::warn!("[search] provider `searxng` has no url; using DuckDuckGo");
                    Some(Self::DuckDuckGo)
                }
            },
            other => {
                log::warn!("[search] provider `{other}` is not supported ({}); using DuckDuckGo", SEARCH_PROVIDERS.join(", "));
                Some(Self::DuckDuckGo)
            }
        }
    }

    /// The provider's name as `[search] provider` spells it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::DuckDuckGo => "duckduckgo",
            Self::Brave { .. } => "brave",
            Self::Searxng { .. } => "searxng",
            Self::Staan { .. } => "staan",
            Self::Tavily { .. } => "tavily",
            Self::Serper { .. } => "serper",
            Self::Google { .. } => "google",
        }
    }
}

/// Results from DuckDuckGo's plain-HTML page: organic hits only (ads skipped), the target URL
/// taken out of DuckDuckGo's redirect link.
fn parse_duckduckgo_html(html: &str, count: usize) -> Vec<Value> {
    use once_cell::sync::Lazy;
    use regex::Regex;
    static TITLE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"(?s)class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#).expect("title regex"));
    static SNIPPET: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"(?s)class="result__snippet"[^>]*>(.*?)</a>"#).expect("snippet regex"));
    static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").expect("tag regex"));
    let text = |raw: &str| {
        let t = TAG.replace_all(raw, "");
        let t = t
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#x27;", "'")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");
        t.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let target = |href: &str| {
        let href = href.replace("&amp;", "&");
        let absolute = if href.starts_with("//") { format!("https:{href}") } else { href };
        let url = reqwest::Url::parse(&absolute).ok()?;
        let real = url.query_pairs().find(|(k, _)| k == "uddg").map(|(_, v)| v.into_owned()).unwrap_or(absolute);
        (!real.contains("duckduckgo.com/y.js")).then_some(real)
    };
    html.split("<div class=\"result ")
        .skip(1)
        .filter(|block| !block.split('>').next().unwrap_or("").contains("result--ad"))
        .filter_map(|block| {
            let t = TITLE.captures(block)?;
            let url = target(&t[1])?;
            let snippet = SNIPPET.captures(block).map(|c| text(&c[1])).unwrap_or_default();
            Some(json!({ "title": text(&t[2]), "url": url, "snippet": snippet }))
        })
        .take(count)
        .collect()
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
        let hits = |v: &Value, list: &str, title: &str, url: &str, snippet: &str| -> Vec<Value> {
            v[list]
                .as_array()
                .map(|a| a.iter().take(count).map(|r| hit(r[title].as_str(), r[url].as_str(), r[snippet].as_str())).collect())
                .unwrap_or_default()
        };
        match &self.backend {
            SearchBackend::DuckDuckGo => {
                let res = self
                    .http
                    .get("https://html.duckduckgo.com/html/")
                    .query(&[("q", query)])
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                let status = res.status();
                let body = res.text().await.map_err(|e| e.to_string())?;
                let results = parse_duckduckgo_html(&body, count);
                if results.is_empty() && (!status.is_success() || body.contains("anomaly")) {
                    return Err(format!(
                        "DuckDuckGo did not answer ({status}); it limits automated searches. Add a Brave, Staan or Tavily key in Setup for steadier search."
                    ));
                }
                Ok(results)
            }
            SearchBackend::Tavily { api_key } => {
                let res = self
                    .http
                    .post("https://api.tavily.com/search")
                    .bearer_auth(api_key)
                    .json(&json!({ "query": query, "max_results": count }))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("Tavily answered {}", res.status()));
                }
                let v: Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(hits(&v, "results", "title", "url", "content"))
            }
            SearchBackend::Serper { api_key } => {
                let res = self
                    .http
                    .post("https://google.serper.dev/search")
                    .header("X-API-KEY", api_key)
                    .json(&json!({ "q": query, "num": count }))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("Serper answered {}", res.status()));
                }
                let v: Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(hits(&v, "organic", "title", "link", "snippet"))
            }
            SearchBackend::Google { api_key, cx } => {
                let res = self
                    .http
                    .get("https://www.googleapis.com/customsearch/v1")
                    .query(&[("key", api_key.as_str()), ("cx", cx.as_str()), ("q", query), ("num", &count.to_string())])
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
                if !res.status().is_success() {
                    return Err(format!("Google Programmable Search answered {}", res.status()));
                }
                let v: Value = res.json().await.map_err(|e| e.to_string())?;
                Ok(hits(&v, "items", "title", "link", "snippet"))
            }
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
        Ok(ToolOutput::new(json!({ "query": query, "provider": self.backend.name(), "results": results }), None))
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
    async fn agents_get_web_fetch_and_web_search_unless_search_is_off() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = crate::config::Config::default();
        c.memory.episodic_path = dir.path().join("episodic").display().to_string();
        let names = |tools: Vec<(String, String)>| tools.into_iter().map(|(n, _)| n).collect::<Vec<_>>();
        let plain = crate::template::agent::TemplateAgent::new(c.clone()).await.unwrap();
        let tools = names(plain.list_tools().await);
        assert!(tools.contains(&"web_fetch".to_string()) && tools.contains(&"web_search".to_string()), "{tools:?}");
        c.additional.insert("search".into(), json!({ "provider": "off" }));
        let quiet = crate::template::agent::TemplateAgent::new(c).await.unwrap();
        let tools = names(quiet.list_tools().await);
        assert!(!tools.contains(&"web_search".to_string()), "{tools:?}");
    }

    #[test]
    fn search_backend_from_config() {
        let mut c = crate::config::Config::default();
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::DuckDuckGo), "no [search]: DuckDuckGo");
        c.additional.insert("search".into(), json!({ "provider": "off" }));
        assert_eq!(SearchBackend::from_config(&c), None);
        c.additional.insert("search".into(), json!({ "provider": "tavily", "api_key": "t" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Tavily { api_key: "t".into() }));
        c.additional.insert("search".into(), json!({ "provider": "serper", "api_key": "p" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Serper { api_key: "p".into() }));
        c.additional.insert("search".into(), json!({ "provider": "google", "api_key": "g", "cx": "engine" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Google { api_key: "g".into(), cx: "engine".into() }));
        c.additional.insert("search".into(), json!({ "provider": "google", "api_key": "g" }));
        if std::env::var("GOOGLE_CSE_ID").is_err() {
            assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::DuckDuckGo), "google without cx falls back");
        }
        c.additional.insert("search".into(), json!({ "provider": "searxng", "url": "https://search.example/" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Searxng { url: "https://search.example".into() }));
        c.additional.insert("search".into(), json!({ "provider": "brave", "api_key": "k" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Brave { api_key: "k".into() }));
        c.additional.insert("search".into(), json!({ "provider": "staan", "api_key": "s", "market": "de-de" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::Staan { api_key: "s".into(), market: Some("de-de".into()) }));
        c.additional.insert("search".into(), json!({ "provider": "bing", "api_key": "k" }));
        assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::DuckDuckGo), "unknown provider falls back");
        if std::env::var("TAVILY_API_KEY").is_err() {
            c.additional.insert("search".into(), json!({ "provider": "tavily" }));
            assert_eq!(SearchBackend::from_config(&c), Some(SearchBackend::DuckDuckGo), "keyed provider without a key falls back");
        }
    }

    #[test]
    fn duckduckgo_html_results_skip_ads_and_unwrap_links() {
        let html = r##"<html><body>
<div class="result results_links results_links_deep result--ad ">
  <h2 class="result__title"><a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fduckduckgo.com%2Fy.js%3Fad_domain%3Dexample.com&amp;rut=1">Buy a course</a></h2>
  <a class="result__snippet" href="#">An ad.</a>
</div>
<div class="result results_links results_links_deep web-result ">
  <h2 class="result__title"><a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Frust%2Dlang.org%2F&amp;rut=2">Rust Programming Language</a></h2>
  <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Frust%2Dlang.org%2F">A language empowering <b>everyone</b> to build reliable &amp; efficient software.</a>
</div>
<div class="result results_links results_links_deep web-result ">
  <h2 class="result__title"><a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FRust_(programming_language)&amp;rut=3">Rust (programming language) - Wikipedia</a></h2>
  <a class="result__snippet" href="#">Rust is a general-purpose language.</a>
</div>
</body></html>"##;
        let hits = parse_duckduckgo_html(html, 5);
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!(hits[0]["url"], "https://rust-lang.org/");
        assert_eq!(hits[0]["title"], "Rust Programming Language");
        assert_eq!(hits[0]["snippet"], "A language empowering everyone to build reliable & efficient software.");
        assert_eq!(hits[1]["url"], "https://en.wikipedia.org/wiki/Rust_(programming_language)");
        assert_eq!(parse_duckduckgo_html(html, 1).len(), 1);
    }

    #[tokio::test]
    async fn live_duckduckgo_when_asked() {
        if std::env::var("KOWALSKI_LIVE_WEB").is_err() {
            return; // network check, opt in with KOWALSKI_LIVE_WEB=1
        }
        let tool = WebSearchTool::new(SearchBackend::DuckDuckGo);
        let hits = tool.search("rust programming language", 5).await.unwrap();
        assert!(!hits.is_empty() && hits.iter().all(|h| h["url"].as_str().is_some_and(|u| u.starts_with("http"))), "{hits:?}");
    }
}
