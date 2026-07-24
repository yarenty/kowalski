//! Secure-by-default access control for the HTTP API: a locally generated bearer token
//! required on every `/api/*` request (health stays open) plus a strict CORS allowlist.
//! Opt out explicitly with `--no-auth` / `[server] no_auth = true`.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::path::{Path, PathBuf};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// Env var that overrides the persisted token (also how spawned workers receive it).
/// Name owned by `kowalski_core::config` so server and CLI stay in sync.
pub const TOKEN_ENV: &str = kowalski_core::config::API_TOKEN_ENV;
/// Token file name under the server state dir (`<config-dir>/db/`, beside `db/rookery/`).
const TOKEN_FILE_NAME: &str = "api_token";
/// Routes reachable without a token (liveness only).
const OPEN_PATHS: &[&str] = &["/api/health"];
/// Browser origins allowed by default: the Vite dev UI (`ui/vite.config.ts`).
pub const DEFAULT_CORS_ORIGINS: &[&str] = &["http://localhost:5173", "http://127.0.0.1:5173"];

/// Resolve the API token: `KOWALSKI_API_TOKEN` wins; else read `<state_dir>/api_token`;
/// else generate one and persist it with mode 0600. Returns `(token, newly_generated, path)`.
pub fn resolve_api_token(state_dir: &Path) -> std::io::Result<(String, bool, PathBuf)> {
    let path = state_dir.join(TOKEN_FILE_NAME);
    if let Ok(env) = std::env::var(TOKEN_ENV) {
        let token = env.trim().to_string();
        if !token.is_empty() {
            return Ok((token, false, path));
        }
    }
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let token = existing.trim().to_string();
        if !token.is_empty() {
            return Ok((token, false, path));
        }
    }
    let token = generate_token();
    std::fs::create_dir_all(state_dir)?;
    write_token_file(&path, &token)?;
    Ok((token, true, path))
}

fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn write_token_file(path: &Path, token: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;
    file.write_all(token.as_bytes())?;
    file.write_all(b"\n")
}

/// Middleware: allow `OPEN_PATHS`, otherwise require `Authorization: Bearer <token>` —
/// or `?token=<token>` for SSE/WebSocket clients that cannot set headers.
pub async fn require_token(token: std::sync::Arc<String>, req: Request<Body>, next: Next) -> Response {
    if OPEN_PATHS.contains(&req.uri().path()) || request_authorized(&req, &token) {
        return next.run(req).await;
    }
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({
            "error": "missing or invalid API token",
            "detail": "send `Authorization: Bearer <token>` (or `?token=`); the kowalski server logs the token file location at startup",
        })),
    )
        .into_response()
}

fn request_authorized(req: &Request<Body>, token: &str) -> bool {
    if let Some(value) = req.headers().get(header::AUTHORIZATION)
        && let Ok(s) = value.to_str()
        && let Some(bearer) = s.strip_prefix("Bearer ")
        && constant_time_eq(bearer.trim(), token)
    {
        return true;
    }
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(candidate) = pair.strip_prefix("token=")
                && constant_time_eq(candidate, token)
            {
                return true;
            }
        }
    }
    false
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Strict allowlist CORS when auth is on; the legacy permissive layer only with `--no-auth`.
/// Origins that fail to parse as header values are skipped (a refused origin gets no ACAO header).
pub fn cors_layer(no_auth: bool, origins: &[String]) -> CorsLayer {
    if no_auth {
        return CorsLayer::permissive();
    }
    let list: Vec<axum::http::HeaderValue> = origins
        .iter()
        .filter_map(|o| o.trim().trim_end_matches('/').parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(list))
        .allow_methods(Any)
        .allow_headers(Any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use tower::util::ServiceExt;

    /// Serializes tests that read or mutate `KOWALSKI_API_TOKEN` (process-global state).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_token_env<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var(TOKEN_ENV).ok();
        match value {
            Some(v) => unsafe { std::env::set_var(TOKEN_ENV, v) },
            None => unsafe { std::env::remove_var(TOKEN_ENV) },
        }
        let out = f();
        match prev {
            Some(v) => unsafe { std::env::set_var(TOKEN_ENV, v) },
            None => unsafe { std::env::remove_var(TOKEN_ENV) },
        }
        out
    }

    #[test]
    fn generates_persists_and_reuses_token() {
        with_token_env(None, || {
            let dir = tempfile::tempdir().unwrap();
            let (token, generated, path) = resolve_api_token(dir.path()).unwrap();
            assert!(generated);
            assert_eq!(token.len(), 64);
            assert_eq!(path, dir.path().join("api_token"));
            let (again, generated_again, _) = resolve_api_token(dir.path()).unwrap();
            assert!(!generated_again);
            assert_eq!(again, token);
        });
    }

    #[cfg(unix)]
    #[test]
    fn token_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        with_token_env(None, || {
            let dir = tempfile::tempdir().unwrap();
            let (_, _, path) = resolve_api_token(dir.path()).unwrap();
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        });
    }

    #[test]
    fn env_token_overrides_file() {
        with_token_env(Some("env-token"), || {
            let dir = tempfile::tempdir().unwrap();
            let (token, generated, _) = resolve_api_token(dir.path()).unwrap();
            assert_eq!(token, "env-token");
            assert!(!generated);
            assert!(!dir.path().join("api_token").exists());
        });
    }

    fn authed_request(path: &str, auth: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(path);
        if let Some(a) = auth {
            builder = builder.header(header::AUTHORIZATION, a);
        }
        builder.body(Body::empty()).unwrap()
    }

    #[test]
    fn header_and_query_token_checks() {
        let ok = authed_request("/api/agents", Some("Bearer s3cret"));
        assert!(request_authorized(&ok, "s3cret"));
        let wrong = authed_request("/api/agents", Some("Bearer nope"));
        assert!(!request_authorized(&wrong, "s3cret"));
        let missing = authed_request("/api/agents", None);
        assert!(!request_authorized(&missing, "s3cret"));
        let query = authed_request("/api/federation/stream?topic=federation&token=s3cret", None);
        assert!(request_authorized(&query, "s3cret"));
        let bad_query = authed_request("/api/federation/stream?token=nope", None);
        assert!(!request_authorized(&bad_query, "s3cret"));
    }

    fn test_router(token: &str) -> Router {
        let token = std::sync::Arc::new(token.to_string());
        Router::new()
            .route("/api/health", get(|| async { "ok" }))
            .route("/api/agents", get(|| async { "agents" }))
            .layer(axum::middleware::from_fn(move |req, next| {
                let token = token.clone();
                require_token(token, req, next)
            }))
    }

    #[tokio::test]
    async fn middleware_denies_without_token_and_allows_with() {
        let app = test_router("s3cret");
        let denied = app
            .clone()
            .oneshot(authed_request("/api/agents", None))
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let allowed = app
            .clone()
            .oneshot(authed_request("/api/agents", Some("Bearer s3cret")))
            .await
            .unwrap();
        assert_eq!(allowed.status(), StatusCode::OK);
        let health = app
            .oneshot(authed_request("/api/health", None))
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
    }
}
