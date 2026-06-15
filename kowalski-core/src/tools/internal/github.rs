//! **Internal GitHub / HTTP fetch** — README API + raw blob URLs, optional `GITHUB_TOKEN`.
//!
//! This is the **default, dependency-light** path. Replace with a **GitHub MCP** (in-repo server,
//! stdio MCP, or a [Docker MCP Toolkit](https://docs.docker.com/ai/mcp-catalog-and-toolkit/toolkit/)
//! profile) when you need issues, search, or OAuth — wire-through is via `McpHub` + config, not
//! by growing this module without bound.

use reqwest::blocking::{Client, Response};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 90;
const MAX_BODY_CHARS: usize = 240_000;

/// How the URL was resolved for diagnostics (metadata table + section headers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubFetchKind {
    ReadmeApi,
    RawUserContent,
    PlainHttp,
}

#[derive(Debug, Clone)]
pub struct FetchedUrlBody {
    pub text: String,
    pub kind: GithubFetchKind,
    pub resolved_url: String,
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .user_agent(concat!("Kowalski/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

fn github_token() -> Option<String> {
    std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn apply_github_auth(req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
    if let Some(token) = github_token() {
        req.header("Authorization", format!("Bearer {}", token.trim()))
    } else {
        req
    }
}

/// Strip known `github.com` host prefixes; returns path after `owner/repo`.
fn github_tail(url: &str) -> Option<String> {
    let u = url.trim().trim_end_matches('/');
    for prefix in [
        "https://github.com/",
        "http://github.com/",
        "https://www.github.com/",
        "http://www.github.com/",
    ] {
        if let Some(t) = u.strip_prefix(prefix) {
            return Some(t.to_string());
        }
    }
    None
}

/// `owner/repo` or `owner/repo/...` → README API URL.
fn readme_api_url(owner: &str, repo: &str) -> String {
    format!("https://api.github.com/repos/{owner}/{repo}/readme")
}

/// Convert `https://github.com/o/r/blob/ref/path/to/file` → raw.githubusercontent.com URL.
fn blob_url_to_raw(owner: &str, repo: &str, git_ref: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{git_ref}/{path}")
}

/// Parse GitHub browser URLs into a fetch strategy.
pub fn resolve_github_fetch(url: &str) -> Option<ResolvedGithub> {
    let tail = github_tail(url)?;
    let segments: Vec<&str> = tail.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return None;
    }
    let owner = segments[0];
    let repo = segments[1];

    if segments.len() == 2 {
        return Some(ResolvedGithub::Readme {
            owner: owner.to_string(),
            repo: repo.to_string(),
        });
    }

    match segments.get(2).copied() {
        Some("blob") if segments.len() >= 5 => {
            let git_ref = segments[3];
            let path = segments[4..].join("/");
            Some(ResolvedGithub::RawFile {
                owner: owner.to_string(),
                repo: repo.to_string(),
                git_ref: git_ref.to_string(),
                path,
            })
        }
        Some("raw") if segments.len() >= 5 => {
            let git_ref = segments[3];
            let path = segments[4..].join("/");
            Some(ResolvedGithub::RawFile {
                owner: owner.to_string(),
                repo: repo.to_string(),
                git_ref: git_ref.to_string(),
                path,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum ResolvedGithub {
    Readme { owner: String, repo: String },
    RawFile {
        owner: String,
        repo: String,
        git_ref: String,
        path: String,
    },
}

impl ResolvedGithub {
    fn fetch(&self, client: &Client) -> Result<Response, String> {
        match self {
            ResolvedGithub::Readme { owner, repo } => {
                let u = readme_api_url(owner, repo);
                let req = apply_github_auth(
                    client
                        .get(&u)
                        .header("Accept", "application/vnd.github.raw+json"),
                );
                req.send().map_err(|e| e.to_string())
            }
            ResolvedGithub::RawFile {
                owner,
                repo,
                git_ref,
                path,
            } => {
                let u = blob_url_to_raw(owner, repo, git_ref, path);
                let req = apply_github_auth(client.get(&u));
                req.send().map_err(|e| e.to_string())
            }
        }
    }

    fn resolved_url_display(&self) -> String {
        match self {
            ResolvedGithub::Readme { owner, repo } => readme_api_url(owner, repo),
            ResolvedGithub::RawFile {
                owner,
                repo,
                git_ref,
                path,
            } => blob_url_to_raw(owner, repo, git_ref, path),
        }
    }

    fn fetch_kind(&self) -> GithubFetchKind {
        match self {
            ResolvedGithub::Readme { .. } => GithubFetchKind::ReadmeApi,
            ResolvedGithub::RawFile { .. } => GithubFetchKind::RawUserContent,
        }
    }
}

fn read_response_text(resp: Response) -> Result<String, String> {
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {} {}",
            resp.status().as_u16(),
            resp.status().canonical_reason().unwrap_or("")
        ));
    }
    let text = resp
        .text()
        .map_err(|e| e.to_string())?
        .chars()
        .take(MAX_BODY_CHARS)
        .collect::<String>();
    Ok(text)
}

/// Fetch URL body with GitHub-specific resolution when applicable.
/// Falls back to a plain HTTP GET on the original URL if GitHub-specific fetching fails.
pub fn fetch_url_for_ingest(original_url: &str) -> Result<FetchedUrlBody, String> {
    let client = http_client()?;

    if let Some(resolved) = resolve_github_fetch(original_url) {
        let kind = resolved.fetch_kind();
        let resolved_url = resolved.resolved_url_display();
        if let Ok(resp) = resolved.fetch(&client)
            && let Ok(text) = read_response_text(resp)
        {
            return Ok(FetchedUrlBody {
                text,
                kind,
                resolved_url,
            });
        }
        // GitHub resolution failed; fall through to plain GET of the browser URL.
    }

    let resp = client.get(original_url).send().map_err(|e| e.to_string())?;
    let text = read_response_text(resp)?;
    Ok(FetchedUrlBody {
        text,
        kind: GithubFetchKind::PlainHttp,
        resolved_url: original_url.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_repo_only() {
        let r = resolve_github_fetch("https://github.com/octocat/Hello-World").unwrap();
        match r {
            ResolvedGithub::Readme { owner, repo } => {
                assert_eq!(owner, "octocat");
                assert_eq!(repo, "Hello-World");
            }
            _ => panic!("expected Readme"),
        }
    }

    #[test]
    fn readme_trailing_slash() {
        assert!(resolve_github_fetch("https://github.com/foo/bar/").is_some());
    }

    #[test]
    fn blob_to_raw_resolution() {
        let r = resolve_github_fetch(
            "https://github.com/rust-lang/rust/blob/master/README.md",
        )
        .unwrap();
        match r {
            ResolvedGithub::RawFile {
                owner,
                repo,
                git_ref,
                path,
            } => {
                assert_eq!(owner, "rust-lang");
                assert_eq!(repo, "rust");
                assert_eq!(git_ref, "master");
                assert_eq!(path, "README.md");
            }
            _ => panic!("expected RawFile"),
        }
    }

    #[test]
    fn nested_blob_path() {
        let r = resolve_github_fetch(
            "https://github.com/o/r/blob/main/docs/guide.md",
        )
        .unwrap();
        match r {
            ResolvedGithub::RawFile { path, .. } => assert_eq!(path, "docs/guide.md"),
            _ => panic!("expected RawFile"),
        }
    }

    #[test]
    fn non_github_returns_none() {
        assert!(resolve_github_fetch("https://example.com/page").is_none());
    }

    #[test]
    fn raw_display_url() {
        let r = resolve_github_fetch(
            "https://github.com/a/b/blob/v1.0/Cargo.toml",
        )
        .unwrap();
        assert_eq!(
            r.resolved_url_display(),
            "https://raw.githubusercontent.com/a/b/v1.0/Cargo.toml"
        );
    }
}
