//! First-run setup: the UI asks three questions (which model, which folder, connect tableski?)
//! and this module writes the answers into the config, then restarts the server so every agent
//! picks them up. Nobody has to open a TOML file.
//!
//! - `GET  /api/setup/status`            what is configured, which Ollama models exist
//! - `POST /api/setup/test-model`        check a model choice without spending tokens
//! - `POST /api/setup/save`              write model + folder into the config
//! - `POST /api/setup/tableski/start`    begin the tableski OAuth sign-in (returns the URL)
//! - `GET  /api/setup/tableski/callback` finish it (browser redirect), write the MCP entry
//! - `POST /api/setup/tableski/disconnect`
//! - `POST /api/setup/restart`           re-exec the server with the new config

use crate::http_api::ApiState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use kowalski_core::mcp::oauth;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Default tableski MCP endpoint offered by the setup screen.
pub const TABLESKI_MCP_URL: &str = "https://mcp.tableski.io/";
/// Name of the MCP server entry setup writes for tableski.
pub const TABLESKI_SERVER_NAME: &str = "tableski";

type ApiResult = Result<Json<Value>, (StatusCode, String)>;

fn bad(msg: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.into())
}

/// Where setup writes: the file the server loaded, or, when that does not exist yet and was
/// only the implicit `./config.toml`, the per-user config the next start finds from anywhere.
pub fn write_target(config_path: &Path) -> PathBuf {
    if config_path.is_file() || config_path.is_absolute() && config_path != Path::new("config.toml") {
        return config_path.to_path_buf();
    }
    if config_path == Path::new("config.toml")
        && std::env::var(kowalski_core::config::CONFIG_ENV).is_err()
        && let Some(user) = kowalski_core::config::user_config_path()
    {
        return user;
    }
    config_path.to_path_buf()
}

fn read_table(path: &Path) -> Result<toml::Table, String> {
    match std::fs::read_to_string(path) {
        Ok(raw) => raw.parse::<toml::Table>().map_err(|e| format!("{}: {e}", path.display())),
        Err(_) => Ok(toml::Table::new()),
    }
}

/// Write `table` to `path`, keeping the previous file as `<name>.bak`; owner-only when it holds a key.
fn write_table(path: &Path, table: &toml::Table, secret_inside: bool) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if path.is_file() {
        let _ = std::fs::copy(path, path.with_extension("toml.bak"));
    }
    let body = format!(
        "# Written by kowalski's setup screen. Edit freely; setup keeps your other settings.\n\n{}",
        toml::to_string(table).map_err(|e| e.to_string())?
    );
    std::fs::write(path, body).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    if secret_inside {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = secret_inside;
    Ok(())
}

fn sub<'a>(t: &'a mut toml::Table, key: &str) -> &'a mut toml::Table {
    let entry = t.entry(key.to_string()).or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if !entry.is_table() {
        *entry = toml::Value::Table(toml::Table::new());
    }
    entry.as_table_mut().expect("table")
}

async fn ollama_models(host: &str, port: u16) -> Option<Vec<String>> {
    let url = format!("http://{host}:{port}/api/tags");
    let res = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?
        .get(url)
        .send()
        .await
        .ok()?;
    let v: Value = res.json().await.ok()?;
    Some(
        v["models"]
            .as_array()?
            .iter()
            .filter_map(|m| m["name"].as_str().map(str::to_string))
            .collect(),
    )
}

fn tableski_entry(config: &kowalski_core::config::Config) -> Option<&kowalski_core::config::McpServerConfig> {
    config.mcp.servers.iter().find(|s| s.name == TABLESKI_SERVER_NAME)
}

/// `GET /api/setup/status`.
pub async fn status(State(state): State<ApiState>) -> Json<Value> {
    let cfg = &state.full_config;
    let target = write_target(&state.config_path);
    let models = ollama_models(&cfg.ollama.host, cfg.ollama.port).await;
    let tableski = tableski_entry(cfg);
    Json(json!({
        "configured": state.config_path.is_file(),
        "config_path": state.config_path.display().to_string(),
        "write_path": target.display().to_string(),
        "provider": cfg.llm.provider,
        "model": kowalski_core::config::default_model(cfg),
        "openai_api_base": cfg.llm.openai_api_base,
        "has_api_key": cfg.llm.openai_api_key.as_deref().is_some_and(|k| !k.is_empty()),
        "ollama": { "reachable": models.is_some(), "models": models.unwrap_or_default(),
                    "url": format!("http://{}:{}", cfg.ollama.host, cfg.ollama.port) },
        "files_dir": files_dir(cfg),
        "vault_dir": cfg.additional.get("vault").and_then(|v| v.get("dir")).and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()),
        "web_search": kowalski_core::tools::internal::SearchBackend::from_config(cfg).is_some(),
        "search": search_status(cfg),
        "tableski": {
            "connected": tableski.is_some(),
            "url": tableski.map(|s| s.url.clone()),
            "signed_in": tableski.and_then(|s| s.oauth.as_ref()).is_some(),
        },
    }))
}

/// Which search provider agents use, whether its key is set, and Google's engine id (not secret).
pub(crate) fn search_status(cfg: &kowalski_core::config::Config) -> Value {
    let table = cfg.additional.get("search");
    let field = |k: &str| table.and_then(|t| t.get(k)).and_then(|v| v.as_str()).filter(|v| !v.trim().is_empty());
    let provider = kowalski_core::tools::internal::SearchBackend::from_config(cfg).map_or("off", |b| b.name());
    json!({ "provider": provider, "has_key": field("api_key").is_some(), "engine_id": field("cx") })
}

/// The folder chat's file tool is confined to when a request names none (`[files] dir`).
pub fn files_dir(cfg: &kowalski_core::config::Config) -> Option<String> {
    cfg.additional
        .get("files")
        .and_then(|v| v.get("dir"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

/// A model choice from the setup screen.
#[derive(Deserialize)]
pub struct ModelChoice {
    provider: String,
    model: String,
    #[serde(default)]
    openai_api_base: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    files_dir: Option<String>,
    /// Key for a keyed search provider (`[search] api_key`); empty keeps the saved one.
    #[serde(default)]
    search_api_key: Option<String>,
    /// One of `SEARCH_PROVIDERS` (`duckduckgo`, the no-key default, … `off`).
    #[serde(default)]
    search_provider: Option<String>,
    /// Google Programmable Search's search engine id (`[search] cx`).
    #[serde(default)]
    search_engine_id: Option<String>,
    /// The notes vault (an Obsidian vault folder, `[vault] dir`); empty removes it.
    #[serde(default)]
    vault_dir: Option<String>,
}

/// `POST /api/setup/test-model`: Ollama must list the model; a hosted endpoint must accept the
/// key on its `/models` listing. No completion is requested, so nothing is billed.
pub async fn test_model(State(state): State<ApiState>, Json(c): Json<ModelChoice>) -> ApiResult {
    match c.provider.as_str() {
        "ollama" => {
            let cfg = &state.full_config;
            let Some(models) = ollama_models(&cfg.ollama.host, cfg.ollama.port).await else {
                return Ok(Json(json!({ "ok": false, "message": format!("Ollama is not answering at http://{}:{}. Start it, or pick a hosted model.", cfg.ollama.host, cfg.ollama.port) })));
            };
            let found = models.iter().any(|m| m == &c.model || m.split(':').next() == Some(c.model.as_str()));
            Ok(Json(json!({ "ok": found, "message": if found { "Ollama has this model.".to_string() } else { format!("Ollama does not have `{}` yet: run `ollama pull {}`.", c.model, c.model) } })))
        }
        "openai" => {
            let base = c.openai_api_base.as_deref().unwrap_or("https://api.openai.com/v1").trim_end_matches('/').to_string();
            let key = c.api_key.clone().filter(|k| !k.is_empty()).or_else(|| state.full_config.llm.openai_api_key.clone()).unwrap_or_default();
            let res = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| bad(e.to_string()))?
                .get(format!("{base}/models"))
                .bearer_auth(&key)
                .send()
                .await;
            let res = match res {
                Ok(r) => r,
                Err(e) => return Ok(Json(json!({ "ok": false, "message": format!("Could not reach {base}: {e}") }))),
            };
            let status = res.status();
            if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
                return Ok(Json(json!({ "ok": false, "message": "The endpoint refused the key." })));
            }
            if !status.is_success() {
                return Ok(Json(json!({ "ok": true, "message": format!("The endpoint answered {status} on /models; the key could not be checked, saving anyway works.") })));
            }
            let v: Value = res.json().await.unwrap_or(Value::Null);
            let listed = v["data"].as_array().map(|a| a.iter().any(|m| m["id"].as_str() == Some(c.model.as_str())));
            let message = match listed {
                Some(true) => "Key accepted; the model is available.".to_string(),
                Some(false) => format!("Key accepted, but `{}` is not in the endpoint's model list. Check the name.", c.model),
                None => "Key accepted.".to_string(),
            };
            Ok(Json(json!({ "ok": listed != Some(false), "message": message })))
        }
        other => Err(bad(format!("unknown provider `{other}`"))),
    }
}

/// `POST /api/setup/save`: model (and key) plus the files folder into the config.
pub async fn save(State(state): State<ApiState>, Json(c): Json<ModelChoice>) -> ApiResult {
    if c.model.trim().is_empty() {
        return Err(bad("choose a model"));
    }
    let path = write_target(&state.config_path);
    let mut t = read_table(&path).map_err(bad)?;
    let mut secret = false;
    {
        let llm = sub(&mut t, "llm");
        match c.provider.as_str() {
            "ollama" => {
                llm.insert("provider".into(), "ollama".into());
                llm.remove("model");
            }
            "openai" => {
                llm.insert("provider".into(), "openai".into());
                llm.insert("model".into(), c.model.trim().into());
                if let Some(base) = c.openai_api_base.as_deref().filter(|b| !b.trim().is_empty()) {
                    llm.insert("openai_api_base".into(), base.trim().into());
                }
                if let Some(key) = c.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
                    llm.insert("openai_api_key".into(), key.trim().into());
                }
                secret = llm.contains_key("openai_api_key");
            }
            other => return Err(bad(format!("unknown provider `{other}`"))),
        }
    }
    if c.provider == "ollama" {
        sub(&mut t, "ollama").insert("model".into(), c.model.trim().into());
    }
    if let Some(dir) = c.files_dir.as_deref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            let expanded = expand_home(dir);
            let path = Path::new(&expanded);
            if path.exists() && !path.is_dir() {
                return Err(bad(format!("`{dir}` is a file, not a folder")));
            }
            // A first-time user types the suggested folder before it exists: make it.
            std::fs::create_dir_all(path).map_err(|e| bad(format!("cannot create the folder `{dir}`: {e}")))?;
            sub(&mut t, "files").insert("dir".into(), expanded.into());
        }
    }
    if let Some(dir) = c.vault_dir.as_deref().map(str::trim) {
        if dir.is_empty() {
            t.remove("vault");
        } else {
            // A vault already exists; a typo must not quietly create a folder notes vanish into.
            if !Path::new(&expand_home(dir)).is_dir() {
                return Err(bad(format!("`{dir}` is not a folder: choose your vault's folder (in Obsidian: the vault's location)")));
            }
            sub(&mut t, "vault").insert("dir".into(), dir.into());
        }
    }
    let key = c.search_api_key.as_deref().map(str::trim).filter(|k| !k.is_empty());
    // A key without a provider is what older clients sent for Brave.
    let provider = c
        .search_provider
        .as_deref()
        .map(|p| p.trim().to_ascii_lowercase())
        .filter(|p| !p.is_empty())
        .or_else(|| key.map(|_| "brave".to_string()));
    if let Some(p) = provider.filter(|p| p != "searxng") {
        use kowalski_core::tools::internal::SEARCH_PROVIDERS;
        if !SEARCH_PROVIDERS.contains(&p.as_str()) {
            return Err(bad(format!("unknown search provider `{p}`")));
        }
        let search = sub(&mut t, "search");
        let saved_provider = search.get("provider").and_then(|v| v.as_str()).map(str::to_ascii_lowercase);
        let has_saved_key = saved_provider.as_deref() == Some(p.as_str())
            && search.get("api_key").and_then(|v| v.as_str()).is_some_and(|k| !k.trim().is_empty());
        search.insert("provider".into(), p.clone().into());
        if p == "duckduckgo" || p == "off" {
            search.remove("api_key");
            search.remove("cx");
        } else {
            match key {
                Some(k) => {
                    search.insert("api_key".into(), k.into());
                }
                None if has_saved_key => {}
                None => return Err(bad(format!("paste a {p} key, or choose DuckDuckGo (no key)"))),
            }
            if p == "google" {
                let cx = c.search_engine_id.as_deref().map(str::trim).filter(|v| !v.is_empty());
                match cx {
                    Some(cx) => {
                        search.insert("cx".into(), cx.into());
                    }
                    None if search.get("cx").is_some() => {}
                    None => return Err(bad("Google Programmable Search needs its search engine ID (cx)")),
                }
            } else {
                search.remove("cx");
            }
        }
        secret |= search.contains_key("api_key");
    }
    write_table(&path, &t, secret).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    log::info!("setup: config written to {}", path.display());
    Ok(Json(json!({ "ok": true, "config_path": path.display().to_string(), "restart_needed": true })))
}

use kowalski_core::config::expand_home;

struct Pending {
    server: oauth::AuthServer,
    client_id: String,
    client_secret: Option<String>,
    verifier: String,
    redirect_uri: String,
    resource: String,
    started: std::time::Instant,
}

fn pending() -> &'static Mutex<HashMap<String, Pending>> {
    static P: std::sync::OnceLock<Mutex<HashMap<String, Pending>>> = std::sync::OnceLock::new();
    P.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Body of the tableski connect request.
#[derive(Deserialize, Default)]
pub struct ConnectBody {
    #[serde(default)]
    url: Option<String>,
}

/// `POST /api/setup/tableski/start`: discover, register, and hand the browser the sign-in URL.
pub async fn tableski_start(State(state): State<ApiState>, body: Option<Json<ConnectBody>>) -> ApiResult {
    let url = body.and_then(|Json(b)| b.url).filter(|u| !u.trim().is_empty()).unwrap_or_else(|| TABLESKI_MCP_URL.to_string());
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e| bad(e.to_string()))?;
    let server = oauth::discover(&http, &url).await.map_err(|e| bad(format!("tableski sign-in unavailable: {e}")))?;
    // loopback redirect (what OAuth servers accept for local apps), even when bound to 0.0.0.0
    let redirect_uri = format!(
        "{}/api/setup/tableski/callback",
        state.api_url.trim_end_matches('/').replace("://0.0.0.0", "://127.0.0.1")
    );
    let (client_id, client_secret) = oauth::register_client(&http, &server, &redirect_uri)
        .await
        .map_err(|e| bad(format!("registering with tableski failed: {e}")))?;
    let (verifier, challenge) = oauth::pkce();
    let st = uuid::Uuid::new_v4().simple().to_string();
    let resource = if url.ends_with('/') { url.clone() } else { format!("{url}/") };
    let authorize = oauth::authorize_url(&server, &client_id, &redirect_uri, &challenge, &st, &resource);
    let mut p = pending().lock().unwrap_or_else(|e| e.into_inner());
    p.retain(|_, v| v.started.elapsed() < std::time::Duration::from_secs(900));
    p.insert(st, Pending { server, client_id, client_secret, verifier, redirect_uri, resource, started: std::time::Instant::now() });
    Ok(Json(json!({ "authorize_url": authorize })))
}

/// `GET /api/setup/tableski/callback?code&state`: exchange, store tokens, write the MCP entry,
/// back to the UI. Protected by the one-time `state` from the start step.
pub async fn tableski_callback(State(state): State<ApiState>, Query(q): Query<HashMap<String, String>>) -> Response {
    let back = |outcome: &str| Redirect::to(&format!("/?setup=tableski-{outcome}")).into_response();
    if q.contains_key("error") {
        log::info!("tableski sign-in declined: {:?}", q.get("error"));
        return back("cancelled");
    }
    let (Some(code), Some(st)) = (q.get("code"), q.get("state")) else { return back("failed") };
    let Some(p) = pending().lock().unwrap_or_else(|e| e.into_inner()).remove(st) else { return back("expired") };
    let http = reqwest::Client::new();
    let tokens = match oauth::exchange_code(&http, &p.server, &p.client_id, p.client_secret.as_deref(), code, &p.verifier, &p.redirect_uri, &p.resource).await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("tableski token exchange: {e}");
            return back("failed");
        }
    };
    let path = write_target(&state.config_path);
    let base = path.parent().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let token_file = base.join("db").join("oauth").join(format!("{TABLESKI_SERVER_NAME}.json"));
    if let Err(e) = tokens.save(&token_file) {
        log::warn!("tableski token file: {e}");
        return back("failed");
    }
    let token_file = std::fs::canonicalize(&token_file).unwrap_or(token_file);
    match upsert_mcp_server(&path, &p.resource, Some(&token_file)) {
        Ok(()) => back("connected"),
        Err(e) => {
            log::warn!("tableski config entry: {e}");
            back("failed")
        }
    }
}

fn upsert_mcp_server(path: &Path, url: &str, token_file: Option<&Path>) -> Result<(), String> {
    let mut t = read_table(path)?;
    let mcp = sub(&mut t, "mcp");
    let servers = mcp.entry("servers".to_string()).or_insert_with(|| toml::Value::Array(Vec::new()));
    let arr = servers.as_array_mut().ok_or("mcp.servers is not a list")?;
    arr.retain(|v| v.get("name").and_then(|n| n.as_str()) != Some(TABLESKI_SERVER_NAME));
    if let Some(tf) = token_file {
        let mut e = toml::Table::new();
        e.insert("name".into(), TABLESKI_SERVER_NAME.into());
        e.insert("url".into(), url.into());
        e.insert("transport".into(), "http".into());
        let mut o = toml::Table::new();
        o.insert("token_file".into(), tf.display().to_string().into());
        e.insert("oauth".into(), toml::Value::Table(o));
        arr.push(toml::Value::Table(e));
    }
    let secret = t.get("llm").and_then(|l| l.get("openai_api_key")).is_some();
    write_table(path, &t, secret)
}

/// `POST /api/setup/tableski/disconnect`: drop the entry and the stored tokens.
pub async fn tableski_disconnect(State(state): State<ApiState>) -> ApiResult {
    let path = write_target(&state.config_path);
    if let Some(entry) = tableski_entry(&state.full_config)
        && let Some(o) = &entry.oauth
    {
        let _ = std::fs::remove_file(&o.token_file);
    }
    upsert_mcp_server(&path, "", None).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(json!({ "ok": true, "restart_needed": true })))
}

/// `POST /api/setup/restart`: answer, then replace this process with a fresh one (same
/// arguments, no browser) so the new config reaches every agent. The UI polls `/api/health`.
pub async fn restart() -> Json<Value> {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(e) => {
                log::error!("restart: {e}");
                return;
            }
        };
        let mut args: Vec<String> = std::env::args().skip(1).collect();
        if !args.iter().any(|a| a == "--no-open") {
            args.push("--no-open".into());
        }
        log::info!("restarting with the new configuration");
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let e = std::process::Command::new(&exe).args(&args).exec();
            log::error!("restart failed: {e}");
        }
        #[cfg(not(unix))]
        {
            match std::process::Command::new(&exe).args(&args).spawn() {
                Ok(_) => std::process::exit(0),
                Err(e) => log::error!("restart failed: {e}"),
            }
        }
    });
    Json(json!({ "ok": true, "restarting": true }))
}
