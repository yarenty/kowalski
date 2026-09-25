//! OAuth 2.1 for MCP servers that sign users in instead of taking a static header (the MCP
//! authorization spec): discovery through the server's protected-resource metadata, dynamic
//! client registration with a loopback redirect, the PKCE authorization-code exchange, and a
//! token file whose access token is refreshed (with refresh-token rotation) before it expires.

use crate::error::KowalskiError;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Refresh this long before the access token's expiry.
const REFRESH_MARGIN_SECS: i64 = 60;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn err(msg: impl Into<String>) -> KowalskiError {
    KowalskiError::Configuration(msg.into())
}

/// Everything needed to call the server and keep calling it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenSet {
    /// Registered client id.
    pub client_id: String,
    /// Client secret, when the server issued one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Token endpoint used for refresh.
    pub token_endpoint: String,
    /// Current access token.
    pub access_token: String,
    /// Refresh token (rotated on every refresh when the server rotates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Unix seconds when the access token expires (0 = unknown / no expiry).
    #[serde(default)]
    pub expires_at: i64,
}

impl TokenSet {
    /// Read a token file.
    pub fn load(path: &Path) -> Result<Self, KowalskiError> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| err(format!("OAuth token file {}: {e}", path.display())))?;
        serde_json::from_str(&raw).map_err(|e| err(format!("OAuth token file {}: {e}", path.display())))
    }

    /// Write atomically with owner-only permissions.
    pub fn save(&self, path: &Path) -> Result<(), KowalskiError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| err(e.to_string()))?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default())
            .map_err(|e| err(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&tmp, path).map_err(|e| err(e.to_string()))
    }

    fn apply_token_response(&mut self, v: &Value) -> Result<(), KowalskiError> {
        let access = v["access_token"]
            .as_str()
            .ok_or_else(|| err(format!("token endpoint answered without access_token: {v}")))?;
        self.access_token = access.to_string();
        if let Some(r) = v["refresh_token"].as_str() {
            self.refresh_token = Some(r.to_string());
        }
        self.expires_at = v["expires_in"].as_i64().map(|s| now() + s).unwrap_or(0);
        Ok(())
    }

    fn needs_refresh(&self) -> bool {
        self.expires_at > 0 && self.expires_at - REFRESH_MARGIN_SECS <= now()
    }
}

async fn post_form(http: &reqwest::Client, url: &str, form: &[(&str, &str)]) -> Result<Value, KowalskiError> {
    let body = form
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&");
    let res = http
        .post(url)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(KowalskiError::Request)?;
    let status = res.status();
    let body: Value = res.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(KowalskiError::Network(format!(
            "OAuth token endpoint {url} answered {status}: {}",
            body.get("error_description").or_else(|| body.get("error")).unwrap_or(&body)
        )));
    }
    Ok(body)
}

/// A token file in use by one MCP client: hands out a current bearer token.
#[derive(Debug)]
pub struct OAuthSession {
    path: PathBuf,
    tokens: tokio::sync::Mutex<TokenSet>,
    http: reqwest::Client,
}

impl OAuthSession {
    /// Open the token file named in the server's config.
    pub fn open(path: &Path) -> Result<Self, KowalskiError> {
        Ok(Self {
            path: path.to_path_buf(),
            tokens: tokio::sync::Mutex::new(TokenSet::load(path)?),
            http: reqwest::Client::new(),
        })
    }

    /// A bearer token valid for at least another minute (refreshes first when needed).
    pub async fn bearer(&self) -> Result<String, KowalskiError> {
        let mut t = self.tokens.lock().await;
        if t.needs_refresh() && t.refresh_token.is_some() {
            self.refresh_locked(&mut t).await?;
        }
        Ok(t.access_token.clone())
    }

    /// Refresh now (after the server answered 401) and return the new bearer token.
    pub async fn force_refresh(&self) -> Result<String, KowalskiError> {
        let mut t = self.tokens.lock().await;
        self.refresh_locked(&mut t).await?;
        Ok(t.access_token.clone())
    }

    async fn refresh_locked(&self, t: &mut TokenSet) -> Result<(), KowalskiError> {
        let refresh = t
            .refresh_token
            .clone()
            .ok_or_else(|| err("OAuth session has no refresh token; connect the server again"))?;
        let mut form = vec![("grant_type", "refresh_token"), ("refresh_token", refresh.as_str()), ("client_id", t.client_id.as_str())];
        let secret = t.client_secret.clone();
        if let Some(s) = secret.as_deref() {
            form.push(("client_secret", s));
        }
        let v = post_form(&self.http, &t.token_endpoint, &form).await?;
        t.apply_token_response(&v)?;
        t.save(&self.path)?;
        Ok(())
    }
}

/// An authorization server as its metadata describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthServer {
    /// Issuer identifier.
    pub issuer: String,
    /// Where the browser goes to sign in.
    pub authorization_endpoint: String,
    /// Code and refresh exchanges.
    pub token_endpoint: String,
    /// Dynamic client registration, when offered.
    pub registration_endpoint: Option<String>,
}

fn origin(url: &reqwest::Url) -> String {
    let mut o = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
    if let Some(p) = url.port() {
        o.push_str(&format!(":{p}"));
    }
    o
}

async fn get_json(http: &reqwest::Client, url: &str) -> Option<Value> {
    let res = http.get(url).header("accept", "application/json").send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    res.json().await.ok()
}

/// Find the authorization server of an MCP endpoint: its protected-resource metadata names the
/// issuer, whose metadata lists the endpoints. Falls back to the MCP origin as the issuer.
pub async fn discover(http: &reqwest::Client, mcp_url: &str) -> Result<AuthServer, KowalskiError> {
    let url = reqwest::Url::parse(mcp_url).map_err(|e| err(format!("MCP URL {mcp_url}: {e}")))?;
    let base = origin(&url);
    let path = url.path().trim_end_matches('/');
    let mut candidates = vec![format!("{base}/.well-known/oauth-protected-resource")];
    if !path.is_empty() {
        candidates.insert(0, format!("{base}/.well-known/oauth-protected-resource{path}"));
    }
    let mut issuer = base.clone();
    for c in candidates {
        if let Some(v) = get_json(http, &c).await
            && let Some(first) = v["authorization_servers"].as_array().and_then(|a| a.first()).and_then(Value::as_str)
        {
            issuer = first.trim_end_matches('/').to_string();
            break;
        }
    }
    let meta = match get_json(http, &format!("{issuer}/.well-known/oauth-authorization-server")).await {
        Some(v) => v,
        None => get_json(http, &format!("{issuer}/.well-known/openid-configuration"))
            .await
            .ok_or_else(|| err(format!("no OAuth metadata at {issuer}")))?,
    };
    let s = |k: &str| meta[k].as_str().map(str::to_string);
    Ok(AuthServer {
        issuer: s("issuer").unwrap_or(issuer),
        authorization_endpoint: s("authorization_endpoint").ok_or_else(|| err("metadata without authorization_endpoint"))?,
        token_endpoint: s("token_endpoint").ok_or_else(|| err("metadata without token_endpoint"))?,
        registration_endpoint: s("registration_endpoint"),
    })
}

/// Register kowalski as a public client with `redirect_uri`; returns `(client_id, client_secret)`.
pub async fn register_client(
    http: &reqwest::Client,
    server: &AuthServer,
    redirect_uri: &str,
) -> Result<(String, Option<String>), KowalskiError> {
    let endpoint = server
        .registration_endpoint
        .as_deref()
        .ok_or_else(|| err("this server does not offer dynamic client registration"))?;
    let res = http
        .post(endpoint)
        .json(&serde_json::json!({
            "client_name": "kowalski",
            "redirect_uris": [redirect_uri],
            "token_endpoint_auth_method": "none",
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
        }))
        .send()
        .await
        .map_err(KowalskiError::Request)?;
    let status = res.status();
    let v: Value = res.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(KowalskiError::Network(format!("client registration answered {status}: {v}")));
    }
    let id = v["client_id"].as_str().ok_or_else(|| err("registration without client_id"))?;
    Ok((id.to_string(), v["client_secret"].as_str().map(str::to_string)))
}

/// A PKCE verifier and its S256 challenge.
pub fn pkce() -> (String, String) {
    let verifier = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The URL the browser opens to sign in and consent.
pub fn authorize_url(server: &AuthServer, client_id: &str, redirect_uri: &str, challenge: &str, state: &str, resource: &str) -> String {
    let sep = if server.authorization_endpoint.contains('?') { '&' } else { '?' };
    format!(
        "{}{sep}response_type=code&client_id={}&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}&scope=mcp&resource={}",
        server.authorization_endpoint,
        enc(client_id),
        enc(redirect_uri),
        enc(challenge),
        enc(state),
        enc(resource)
    )
}

/// Exchange the authorization code for tokens.
#[allow(clippy::too_many_arguments)] // mirrors the fields of the token request
pub async fn exchange_code(
    http: &reqwest::Client,
    server: &AuthServer,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
    resource: &str,
) -> Result<TokenSet, KowalskiError> {
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("resource", resource),
    ];
    if let Some(s) = client_secret {
        form.push(("client_secret", s));
    }
    let v = post_form(http, &server.token_endpoint, &form).await?;
    let mut t = TokenSet {
        client_id: client_id.to_string(),
        client_secret: client_secret.map(str::to_string),
        token_endpoint: server.token_endpoint.clone(),
        access_token: String::new(),
        refresh_token: None,
        expires_at: 0,
    };
    t.apply_token_response(&v)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_pair_is_s256_of_the_verifier() {
        let (v, c) = pkce();
        assert!((43..=128).contains(&v.len()));
        assert_eq!(c, URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())));
        assert_ne!(pkce().0, v);
    }

    #[test]
    fn token_file_round_trip_is_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("oauth/t.json");
        let t = TokenSet {
            client_id: "c".into(),
            client_secret: None,
            token_endpoint: "https://x/oauth/token".into(),
            access_token: "a".into(),
            refresh_token: Some("r".into()),
            expires_at: 42,
        };
        t.save(&p).unwrap();
        assert_eq!(TokenSet::load(&p).unwrap(), t);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn authorize_url_carries_pkce_and_resource() {
        let s = AuthServer {
            issuer: "https://mcp.example".into(),
            authorization_endpoint: "https://mcp.example/oauth/authorize".into(),
            token_endpoint: "https://mcp.example/oauth/token".into(),
            registration_endpoint: None,
        };
        let u = authorize_url(&s, "cid", "http://127.0.0.1:3456/cb", "chal", "st", "https://mcp.example/");
        assert!(u.starts_with("https://mcp.example/oauth/authorize?response_type=code&client_id=cid"));
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A3456%2Fcb"));
        assert!(u.contains("code_challenge_method=S256") && u.contains("resource=https%3A%2F%2Fmcp.example%2F"));
    }
}
