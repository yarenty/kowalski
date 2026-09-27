//! Workbooks on the connected tableski account, managed from kowalski: list, upload, remove.
//!
//! The hosted tableski takes files at its HTTP API (`/v1/files`) with the same bearer token its
//! MCP server accepts, so kowalski reuses the token from Setup's "Connect tableski" (refreshed
//! when due) and forwards the request. A local tableski serves the files it was started with and
//! has no upload API; these routes say so instead of failing obscurely.

use crate::http_api::ApiState;
use crate::setup::TABLESKI_SERVER_NAME;
use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use serde_json::{Value, json};

type ApiResult = Result<Json<Value>, (StatusCode, String)>;

/// Biggest workbook kowalski forwards; the account's plan may allow less (tableski says so).
pub const MAX_UPLOAD_BYTES: usize = 512 * 1024 * 1024;

/// The tableski HTTP API behind an MCP URL: `https://mcp.<domain>/` → `https://api.<domain>`.
/// `None` for anything else (a local tableski has no upload API).
pub fn api_base(mcp_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(mcp_url).ok()?;
    let host = url.host_str()?;
    let domain = host.strip_prefix("mcp.")?;
    Some(format!("{}://api.{domain}", url.scheme()))
}

/// API base and a fresh bearer token for the connected tableski, or why there is none.
async fn connection(state: &ApiState) -> Result<(String, String), (StatusCode, String)> {
    let entry = state
        .full_config
        .mcp
        .servers
        .iter()
        .find(|s| s.name == TABLESKI_SERVER_NAME)
        .ok_or((StatusCode::CONFLICT, "tableski is not connected: connect it in Setup first".to_string()))?;
    let base = api_base(&entry.url).ok_or((
        StatusCode::CONFLICT,
        format!(
            "this tableski ({}) runs locally and has no upload: start it with your file (`tableski --file report.xlsx`)",
            entry.url
        ),
    ))?;
    let oauth = entry.oauth.as_ref().ok_or((
        StatusCode::CONFLICT,
        "tableski is connected without sign-in: reconnect it in Setup to upload files".to_string(),
    ))?;
    let session = kowalski_core::mcp::oauth::OAuthSession::open(&oauth.token_file)
        .map_err(|e| (StatusCode::CONFLICT, format!("tableski sign-in unreadable, reconnect in Setup: {e}")))?;
    let token = session
        .bearer()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("tableski sign-in could not be refreshed, reconnect in Setup: {e}")))?;
    Ok((base, token))
}

/// Pass tableski's answer through: its JSON on success, its message (with its status) otherwise.
async fn relay(res: reqwest::Response) -> ApiResult {
    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let body = res.text().await.unwrap_or_default();
    if status.is_success() {
        let value = if body.trim().is_empty() { json!({ "ok": true }) } else { serde_json::from_str(&body).unwrap_or(json!({ "text": body })) };
        return Ok(Json(value));
    }
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v.get("error").or_else(|| v.get("message")).and_then(Value::as_str).map(str::to_string))
        .unwrap_or(body);
    Err((status, format!("tableski: {message}")))
}

fn upstream(e: reqwest::Error) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, format!("tableski unreachable: {e}"))
}

/// `GET /api/tableski/files`: the account's files, their tables, usage and plan limits.
pub async fn list(State(state): State<ApiState>) -> ApiResult {
    let (base, token) = connection(&state).await?;
    let res = reqwest::Client::new()
        .get(format!("{base}/v1/files"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(upstream)?;
    relay(res).await
}

/// `POST /api/tableski/files`, multipart with one `file` part: the workbook goes to tableski,
/// which answers with the tables it made from it.
pub async fn upload(State(state): State<ApiState>, mut multipart: Multipart) -> ApiResult {
    let (base, token) = connection(&state).await?;
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(f)) if f.name() == Some("file") => break f,
            Ok(Some(_)) => continue,
            Ok(None) => return Err((StatusCode::BAD_REQUEST, "send the workbook as a `file` part".to_string())),
            Err(e) => return Err((StatusCode::BAD_REQUEST, e.to_string())),
        }
    };
    let name = field
        .file_name()
        .map(|n| n.rsplit(['/', '\\']).next().unwrap_or(n).to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "upload.xlsx".to_string());
    let bytes = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "that file is larger than 512 MB".to_string()));
    }
    let part = reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(name.clone());
    let form = reqwest::multipart::Form::new().part("file", part);
    log::info!("tableski: uploading {name} ({} bytes)", bytes.len());
    let res = reqwest::Client::new()
        .post(format!("{base}/v1/files"))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(upstream)?;
    relay(res).await
}

/// `DELETE /api/tableski/files/{id}`.
pub async fn remove(State(state): State<ApiState>, Path(id): Path<String>) -> ApiResult {
    if !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err((StatusCode::BAD_REQUEST, "not a file id".to_string()));
    }
    let (base, token) = connection(&state).await?;
    let res = reqwest::Client::new()
        .delete(format!("{base}/v1/files/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(upstream)?;
    relay(res).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_sits_beside_the_mcp_server() {
        assert_eq!(api_base("https://mcp.tableski.io/").as_deref(), Some("https://api.tableski.io"));
        assert_eq!(api_base("http://127.0.0.1:8088/"), None, "a local tableski has no upload API");
        assert_eq!(api_base("not a url"), None);
    }
}
