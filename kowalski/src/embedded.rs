//! What the build compiled into this binary: the operator UI and the built-in hordes.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

/// Directory name of the built-in hordes beside the config (a catalog root, scanned last so a
/// user's own horde with the same id wins).
pub const BUILTIN_HORDES_DIR: &str = "builtin-hordes";

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "json" | "map" => "application/json",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn file(path: &str) -> Option<&'static [u8]> {
    UI_FILES.iter().find(|(p, _)| *p == path).map(|(_, b)| *b)
}

/// Serve the embedded UI for every path the API does not own. Unknown paths without an
/// extension get `index.html` (the app's own routes); hashed assets are cached for a year.
pub async fn serve_ui(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "no such API route").into_response();
    }
    if UI_FILES.is_empty() {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            "<!doctype html><meta charset=utf-8><title>kowalski</title><body style=\"font:16px system-ui;max-width:40rem;margin:3rem auto;padding:0 1rem\"><h1>kowalski is running</h1><p>This build has no operator UI inside. Build it once and rebuild the server:</p><pre>cd ui &amp;&amp; bun install &amp;&amp; bun run build\ncargo build --release -p kowalski</pre><p>The API is up at <code>/api/health</code>.</p></body>",
        )
            .into_response();
    }
    let (name, bytes) = match (path, file(path)) {
        ("", _) => ("index.html", file("index.html")),
        (p, Some(b)) => (p, Some(b)),
        (p, None) if !p.rsplit('/').next().unwrap_or("").contains('.') => ("index.html", file("index.html")),
        (p, None) => (p, None),
    };
    let Some(bytes) = bytes else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let mut res = Response::new(Body::from(bytes));
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type(name)));
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if name.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    res
}

/// Write the built-in hordes under `dir` (refreshed on every start; run output in each
/// horde's workdir is left alone). Returns how many hordes were written.
pub fn install_builtin_hordes(dir: &Path) -> std::io::Result<usize> {
    let mut hordes = std::collections::BTreeSet::new();
    for (rel, bytes) in BUILTIN_HORDE_FILES {
        let target = dir.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if std::fs::read(&target).ok().as_deref() != Some(*bytes) {
            std::fs::write(&target, bytes)?;
        }
        if let Some(id) = rel.split('/').next() {
            hordes.insert(id.to_string());
        }
    }
    Ok(hordes.len())
}

/// Whether this binary carries the UI.
pub fn has_ui() -> bool {
    !UI_FILES.is_empty()
}
