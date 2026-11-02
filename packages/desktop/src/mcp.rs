//! An MCP server on 127.0.0.1 that lets AI clients (Claude Code and others)
//! list saved hosts and run commands on them. Credentials, jump hosts and
//! host trust stay here; every command waits for the user's approval in the
//! app before it runs.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use diesel::prelude::*;
use orm::models::{SshHost, SshKey};
use orm::schema::{settings, ssh_hosts, ssh_keys};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::{oneshot, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

use crate::runtime::ssh::{auth_method, exec_command, ExecOutput, ExecTarget, SshHopInput};
use crate::store::{internal_error, run_db, DbPool};
use crate::trust::SshTrustStore;

const SETTING_KEY: &str = "noverterm-mcp";
const DEFAULT_PORT: u16 = 47321;
const KEYRING_ACCOUNT: &str = "mcp-token";
/// An unanswered approval request is denied after this long.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_RUN_TIMEOUT_SECS: u64 = 60;
const MAX_RUN_TIMEOUT_SECS: u64 = 600;
const MAX_JUMP_HOPS: usize = 5;
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

#[derive(Default)]
pub struct McpState {
    server: Mutex<Option<RunningServer>>,
    pending: StdMutex<HashMap<String, oneshot::Sender<McpDecision>>>,
    /// Hosts the user allowed for every command until the app restarts.
    allowed_hosts: StdMutex<HashSet<String>>,
}

struct RunningServer {
    port: u16,
    shutdown: oneshot::Sender<()>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct McpSettings {
    pub enabled: bool,
    pub port: u16,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct McpStatus {
    pub settings: McpSettings,
    pub running: bool,
    pub url: String,
    /// The bearer token clients must send; present once the server was enabled.
    pub token: Option<String>,
    /// Why the server is not running although it is enabled.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum McpDecision {
    Allow,
    /// Allow this and later commands on the same host until the app restarts.
    AllowHost,
    Deny,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct McpApprovalRequest {
    pub id: String,
    pub host_name: String,
    pub target: String,
    pub command: String,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct McpActivity {
    pub host_name: String,
    pub command: String,
    /// `denied`, `failed`, or `finished`.
    pub status: String,
    pub exit_status: Option<u32>,
}

#[tauri::command]
#[specta::specta]
pub async fn mcp_status(app: AppHandle) -> Result<McpStatus, String> {
    status(&app, None).await
}

#[tauri::command]
#[specta::specta]
pub async fn mcp_configure(app: AppHandle, settings: McpSettings) -> Result<McpStatus, String> {
    let pool = app.state::<DbPool>().inner().clone();
    let value = serde_json::to_string(&settings).map_err(|error| error.to_string())?;
    run_db(pool, move |db| {
        let now = chrono::Utc::now().naive_utc();
        diesel::insert_into(settings::table)
            .values(&orm::models::NewSetting {
                key: SETTING_KEY.to_string(),
                value,
                created_at: now,
                updated_at: now,
            })
            .on_conflict(settings::key)
            .do_update()
            .set((
                settings::value.eq(diesel::upsert::excluded(settings::value)),
                settings::updated_at.eq(now),
            ))
            .execute(db)
            .map(|_| ())
            .map_err(internal_error)
    })
    .await?;
    let error = apply(&app, settings).await.err();
    status(&app, error).await
}

#[tauri::command]
#[specta::specta]
pub async fn mcp_respond(
    state: tauri::State<'_, McpState>,
    request_id: String,
    decision: McpDecision,
) -> Result<(), String> {
    if let Some(sender) = state.pending.lock().unwrap().remove(&request_id) {
        let _ = sender.send(decision);
    }
    Ok(())
}

/// Starts the server at launch when the user left it enabled.
pub async fn start_if_enabled<R: Runtime>(app: AppHandle<R>) {
    match load_settings(&app).await {
        Ok(settings) if settings.enabled => {
            if let Err(error) = apply(&app, settings).await {
                warn!(error, "MCP server did not start");
            }
        }
        Ok(_) => {}
        Err(error) => warn!(error, "Could not read MCP settings"),
    }
}

async fn status<R: Runtime>(
    app: &AppHandle<R>,
    error: Option<String>,
) -> Result<McpStatus, String> {
    let settings = load_settings(app).await?;
    let running = app.state::<McpState>().server.lock().await.is_some();
    Ok(McpStatus {
        settings,
        running,
        url: format!("http://127.0.0.1:{}/mcp", settings.port),
        token: keyring_entry()?.get_password().ok(),
        error,
    })
}

async fn load_settings<R: Runtime>(app: &AppHandle<R>) -> Result<McpSettings, String> {
    let pool = app.state::<DbPool>().inner().clone();
    let value = run_db(pool, |db| {
        settings::table
            .filter(settings::key.eq(SETTING_KEY))
            .select(settings::value)
            .first::<String>(db)
            .optional()
            .map_err(internal_error)
    })
    .await?;
    Ok(value
        .and_then(|value| serde_json::from_str(&value).ok())
        .unwrap_or_default())
}

/// Stops any running server, then starts one on `settings.port` if enabled.
async fn apply<R: Runtime>(app: &AppHandle<R>, settings: McpSettings) -> Result<(), String> {
    let state = app.state::<McpState>();
    let mut server = state.server.lock().await;
    if let Some(running) = server.take() {
        let _ = running.shutdown.send(());
        info!(port = running.port, "MCP server stopped");
    }
    if !settings.enabled {
        return Ok(());
    }

    let token = match keyring_entry()?.get_password() {
        Ok(token) => token,
        Err(_) => {
            let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
            keyring_entry()?
                .set_password(&token)
                .map_err(|error| format!("Could not save the MCP token: {error}"))?;
            token
        }
    };
    *server = Some(start_server(app, settings.port, token).await?);
    Ok(())
}

async fn start_server<R: Runtime>(
    app: &AppHandle<R>,
    port: u16,
    token: String,
) -> Result<RunningServer, String> {
    // Loopback only: nothing off this machine can reach it.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| format!("Could not listen on port {port}: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let router = Router::new()
        .route(
            "/mcp",
            post(handle::<R>).get(not_allowed).delete(not_allowed),
        )
        .with_state(Arc::new(Context {
            app: app.clone(),
            token,
        }));
    let (shutdown, stopped) = oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        let serve = axum::serve(listener, router).with_graceful_shutdown(async {
            let _ = stopped.await;
        });
        if let Err(error) = serve.await {
            warn!(%error, "MCP server exited");
        }
    });
    info!(port, "MCP server listening");
    Ok(RunningServer { port, shutdown })
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("noverterm", KEYRING_ACCOUNT).map_err(|error| error.to_string())
}

struct Context<R: Runtime> {
    app: AppHandle<R>,
    token: String,
}

async fn not_allowed() -> StatusCode {
    // No server-initiated stream: clients fall back to plain POST replies.
    StatusCode::METHOD_NOT_ALLOWED
}

async fn handle<R: Runtime>(
    State(ctx): State<Arc<Context<R>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // A web page in the browser can reach loopback; refuse anything it sends.
    if let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        let local = ["http://127.0.0.1", "http://localhost"]
            .iter()
            .any(|allowed| origin == *allowed || origin.starts_with(&format!("{allowed}:")));
        if !local {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_eq(presented.as_bytes(), ctx.token.as_bytes()) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let Ok(message) = serde_json::from_slice::<Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(rpc_error(Value::Null, -32700, "Parse error")),
        )
            .into_response();
    };
    // Notifications (no id) need no reply.
    let Some(id) = message.get("id").cloned() else {
        return StatusCode::ACCEPTED.into_response();
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let reply = match message["method"].as_str().unwrap_or_default() {
        "initialize" => {
            let requested = params["protocolVersion"].as_str().unwrap_or_default();
            let version = PROTOCOL_VERSIONS
                .iter()
                .find(|version| **version == requested)
                .unwrap_or(&PROTOCOL_VERSIONS[0]);
            rpc_result(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "noverterm", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "Run shell commands on the user's saved SSH hosts. \
                        Call list_hosts first. Every command needs the user's approval in noverterm, \
                        so explain why you run it and prefer read-only commands.",
                }),
            )
        }
        "ping" => rpc_result(id, json!({})),
        "tools/list" => rpc_result(id, json!({ "tools": tools() })),
        "tools/call" => {
            let (text, is_error) = match call_tool(&ctx.app, &params).await {
                Ok(text) => (text, false),
                Err(text) => (text, true),
            };
            rpc_result(
                id,
                json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }),
            )
        }
        method => rpc_error(id, -32601, &format!("Method not found: {method}")),
    };
    Json(reply).into_response()
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn rpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

fn tools() -> Value {
    json!([
        {
            "name": "list_hosts",
            "description": "List the SSH hosts saved in noverterm: name, user@host:port, and any jump hosts it connects through.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true },
        },
        {
            "name": "run_command",
            "description": "Run a non-interactive shell command on a saved host and return its exit status, stdout and stderr. \
                The user approves each command in noverterm first. No TTY: avoid commands that prompt for input.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "host": { "type": "string", "description": "Host name from list_hosts." },
                    "command": { "type": "string", "description": "Shell command to run." },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Stop the command after this many seconds (default 60, max 600).",
                    },
                },
                "required": ["host", "command"],
            },
            "annotations": { "destructiveHint": true, "openWorldHint": true },
        },
    ])
}

async fn call_tool<R: Runtime>(app: &AppHandle<R>, params: &Value) -> Result<String, String> {
    let arguments = &params["arguments"];
    match params["name"].as_str().unwrap_or_default() {
        "list_hosts" => list_hosts(app).await,
        "run_command" => {
            let host = arguments["host"].as_str().unwrap_or_default().trim();
            let command = arguments["command"].as_str().unwrap_or_default().trim();
            if host.is_empty() || command.is_empty() {
                return Err("Both host and command are required".to_string());
            }
            let run_timeout = arguments["timeout_secs"]
                .as_u64()
                .unwrap_or(DEFAULT_RUN_TIMEOUT_SECS)
                .clamp(1, MAX_RUN_TIMEOUT_SECS);
            run_command(app, host, command, Duration::from_secs(run_timeout)).await
        }
        name => Err(format!("Unknown tool: {name}")),
    }
}

async fn list_hosts<R: Runtime>(app: &AppHandle<R>) -> Result<String, String> {
    let (hosts, _) = load_hosts(app).await?;
    let by_id: HashMap<&str, &SshHost> =
        hosts.iter().map(|host| (host.id.as_str(), host)).collect();
    let listed: Vec<Value> = hosts
        .iter()
        .map(|host| {
            let via: Vec<&str> = jump_chain(host, &by_id)
                .map(|chain| chain.iter().map(|hop| hop.name.as_str()).collect())
                .unwrap_or_default();
            json!({
                "name": host.name,
                "target": format!("{}@{}:{}", host.username, host.host, host.port),
                "via": via,
            })
        })
        .collect();
    serde_json::to_string_pretty(&listed).map_err(|error| error.to_string())
}

async fn run_command<R: Runtime>(
    app: &AppHandle<R>,
    host_ref: &str,
    command: &str,
    run_timeout: Duration,
) -> Result<String, String> {
    let (hosts, keys) = load_hosts(app).await?;
    let host = find_host(&hosts, host_ref)?;
    let by_id: HashMap<&str, &SshHost> =
        hosts.iter().map(|host| (host.id.as_str(), host)).collect();
    let chain = jump_chain(host, &by_id)?;
    let jumps = chain
        .iter()
        .map(|hop| {
            let (password, private_key, passphrase) = credentials(hop, &keys);
            Ok(SshHopInput {
                host: hop.host.clone(),
                port: port(hop)?,
                username: hop.username.clone(),
                password,
                private_key,
                passphrase,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let (password, private_key, passphrase) = credentials(host, &keys);
    let auth = auth_method(password, private_key, passphrase)
        .map_err(|_| format!("{} has no saved password or SSH key", host.name))?;

    let activity = |status: &str, exit_status: Option<u32>| {
        let _ = app.emit(
            "mcp_activity",
            McpActivity {
                host_name: host.name.clone(),
                command: command.to_string(),
                status: status.to_string(),
                exit_status,
            },
        );
    };
    if !approve(app, host, command).await {
        activity("denied", None);
        return Err("The user denied this command in noverterm.".to_string());
    }

    let target = ExecTarget {
        host: host.host.clone(),
        port: port(host)?,
        username: host.username.clone(),
        auth,
        jumps,
    };
    let trust_store = app.state::<SshTrustStore>().inner().clone();
    match exec_command(&trust_store, target, command, run_timeout).await {
        Ok(output) => {
            activity("finished", output.exit_status);
            Ok(format_output(&output))
        }
        Err(error) => {
            activity("failed", None);
            Err(error)
        }
    }
}

/// Asks the user in the app; `true` once they allow it.
async fn approve<R: Runtime>(app: &AppHandle<R>, host: &SshHost, command: &str) -> bool {
    let state = app.state::<McpState>();
    if state.allowed_hosts.lock().unwrap().contains(&host.id) {
        return true;
    }
    let id = Uuid::new_v4().to_string();
    let (sender, receiver) = oneshot::channel();
    state.pending.lock().unwrap().insert(id.clone(), sender);
    let request = McpApprovalRequest {
        id: id.clone(),
        host_name: host.name.clone(),
        target: format!("{}@{}:{}", host.username, host.host, host.port),
        command: command.to_string(),
    };
    if app.emit("mcp_approval_request", request).is_err() {
        state.pending.lock().unwrap().remove(&id);
        return false;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }

    let decision = tokio::time::timeout(APPROVAL_TIMEOUT, receiver).await;
    state.pending.lock().unwrap().remove(&id);
    match decision {
        Ok(Ok(McpDecision::Allow)) => true,
        Ok(Ok(McpDecision::AllowHost)) => {
            state.allowed_hosts.lock().unwrap().insert(host.id.clone());
            true
        }
        _ => false,
    }
}

fn format_output(output: &ExecOutput) -> String {
    let mut text = match output.exit_status {
        Some(status) => format!("exit status: {status}\n"),
        None => "exit status: unknown\n".to_string(),
    };
    if !output.stdout.is_empty() {
        text.push_str(&format!("--- stdout ---\n{}\n", output.stdout.trim_end()));
    }
    if !output.stderr.is_empty() {
        text.push_str(&format!("--- stderr ---\n{}\n", output.stderr.trim_end()));
    }
    if output.truncated {
        text.push_str("(output truncated)\n");
    }
    text
}

async fn load_hosts<R: Runtime>(app: &AppHandle<R>) -> Result<(Vec<SshHost>, Vec<SshKey>), String> {
    let pool = app.state::<DbPool>().inner().clone();
    run_db(pool, |db| {
        let hosts = ssh_hosts::table
            .order(ssh_hosts::name.asc())
            .load::<SshHost>(db)
            .map_err(internal_error)?;
        let keys = ssh_keys::table.load::<SshKey>(db).map_err(internal_error)?;
        Ok((hosts, keys))
    })
    .await
}

fn find_host<'a>(hosts: &'a [SshHost], host_ref: &str) -> Result<&'a SshHost, String> {
    if let Some(host) = hosts.iter().find(|host| host.id == host_ref) {
        return Ok(host);
    }
    let matches: Vec<&SshHost> = hosts
        .iter()
        .filter(|host| host.name.eq_ignore_ascii_case(host_ref))
        .collect();
    match matches.as_slice() {
        [host] => Ok(host),
        [] => Err(format!(
            "No saved host named {host_ref}. Call list_hosts to see them."
        )),
        _ => Err(format!(
            "More than one saved host is named {host_ref}; rename one in noverterm."
        )),
    }
}

/// Jump hosts in connect order, outermost first.
fn jump_chain<'a>(
    host: &SshHost,
    by_id: &HashMap<&str, &'a SshHost>,
) -> Result<Vec<&'a SshHost>, String> {
    let mut chain = Vec::new();
    let mut next = host.jump_host_id.as_deref();
    while let Some(id) = next {
        if chain.len() == MAX_JUMP_HOPS {
            return Err(format!("{}'s jump host chain is too long", host.name));
        }
        let hop = by_id
            .get(id)
            .ok_or_else(|| format!("{}'s jump host no longer exists", host.name))?;
        chain.push(*hop);
        next = hop.jump_host_id.as_deref();
    }
    chain.reverse();
    Ok(chain)
}

fn credentials(
    host: &SshHost,
    keys: &[SshKey],
) -> (Option<String>, Option<String>, Option<String>) {
    let key = host
        .ssh_key_id
        .as_ref()
        .and_then(|id| keys.iter().find(|key| &key.id == id));
    (
        host.password.clone(),
        key.map(|key| key.private_key.clone()),
        key.and_then(|key| key.passphrase.clone()),
    )
}

fn port(host: &SshHost) -> Result<u16, String> {
    u16::try_from(host.port).map_err(|_| format!("{} has an invalid port", host.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(id: &str, jump: Option<&str>) -> SshHost {
        let now = chrono::Utc::now().naive_utc();
        SshHost {
            id: id.to_string(),
            name: id.to_string(),
            host: format!("{id}.test"),
            port: 22,
            username: "ops".to_string(),
            ssh_key_id: None,
            password: None,
            group_id: None,
            created_at: now,
            updated_at: now,
            jump_host_id: jump.map(str::to_string),
        }
    }

    #[test]
    fn jump_chain_runs_outermost_first_and_stops_loops() {
        let hosts = [
            host("edge", None),
            host("inner", Some("edge")),
            host("db", Some("inner")),
        ];
        let by_id: HashMap<&str, &SshHost> =
            hosts.iter().map(|host| (host.id.as_str(), host)).collect();
        let chain = jump_chain(&hosts[2], &by_id).unwrap();
        assert_eq!(
            chain.iter().map(|hop| hop.id.as_str()).collect::<Vec<_>>(),
            ["edge", "inner"]
        );

        let looping = [host("a", Some("b")), host("b", Some("a"))];
        let by_id: HashMap<&str, &SshHost> = looping
            .iter()
            .map(|host| (host.id.as_str(), host))
            .collect();
        assert!(jump_chain(&looping[0], &by_id).is_err());
    }

    #[test]
    fn hosts_resolve_by_id_or_unique_name() {
        let hosts = [host("web", None), host("db", None)];
        assert_eq!(find_host(&hosts, "WEB").unwrap().id, "web");
        assert!(find_host(&hosts, "cache").is_err());
    }

    struct TestServer {
        app: tauri::App<tauri::test::MockRuntime>,
        url: String,
        _server: RunningServer,
        _directory: tempfile::TempDir,
    }

    async fn test_server() -> TestServer {
        let directory = tempfile::tempdir().unwrap();
        let app = tauri::test::mock_app();
        let pool = crate::store::test_pool(&directory);
        app.manage(SshTrustStore::new(pool.clone()));
        app.manage(pool);
        app.manage(McpState::default());
        let server = start_server(app.handle(), 0, "secret".to_string())
            .await
            .unwrap();
        TestServer {
            url: format!("http://127.0.0.1:{}/mcp", server.port),
            app,
            _server: server,
            _directory: directory,
        }
    }

    async fn rpc(server: &TestServer, method: &str, params: Value) -> Value {
        reqwest::Client::new()
            .post(&server.url)
            .bearer_auth("secret")
            .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    fn insert_host(server: &TestServer, name: &str, host: &str, port: i32, password: &str) {
        let now = chrono::Utc::now().naive_utc();
        let pool = server.app.state::<DbPool>();
        diesel::insert_into(ssh_hosts::table)
            .values(&orm::models::NewSshHost {
                id: name.to_string(),
                name: name.to_string(),
                host: host.to_string(),
                port,
                username: "root".to_string(),
                ssh_key_id: None,
                password: Some(password.to_string()),
                group_id: None,
                created_at: now,
                updated_at: now,
                jump_host_id: None,
            })
            .execute(&mut pool.get().unwrap())
            .unwrap();
    }

    #[tokio::test]
    async fn speaks_mcp_and_rejects_strangers() {
        let server = test_server().await;
        let client = reqwest::Client::new();

        let anonymous = client
            .post(&server.url)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(anonymous.status(), 401);
        let web_page = client
            .post(&server.url)
            .bearer_auth("secret")
            .header("Origin", "https://evil.example")
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(web_page.status(), 403);
        let notification = client
            .post(&server.url)
            .bearer_auth("secret")
            .json(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .send()
            .await
            .unwrap();
        assert_eq!(notification.status(), 202);

        let init = rpc(
            &server,
            "initialize",
            json!({ "protocolVersion": "2025-03-26" }),
        )
        .await;
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        let tools = rpc(&server, "tools/list", json!({})).await;
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 2);

        insert_host(&server, "web", "web.test", 22, "pw");
        let listed = rpc(&server, "tools/call", json!({ "name": "list_hosts" })).await;
        let text = listed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("root@web.test:22"), "{text}");
        assert!(!text.contains("pw"), "credentials never leave: {text}");
    }

    /// Needs an sshd; see the env vars below. Run with `--ignored`.
    #[tokio::test]
    #[ignore]
    async fn runs_an_approved_command_over_ssh() {
        use tauri::Listener;
        let env = |name: &str| std::env::var(name).unwrap();
        let server = test_server().await;
        let port: i32 = env("MCP_E2E_PORT").parse().unwrap();
        insert_host(&server, "box", "127.0.0.1", port, &env("MCP_E2E_PASSWORD"));
        let call = |command: &str| json!({ "name": "run_command", "arguments": { "host": "box", "command": command } });

        // Stand in for the user: deny `echo hi`, allow everything else.
        let handle = server.app.handle().clone();
        server.app.listen("mcp_approval_request", move |event| {
            let request: Value = serde_json::from_str(event.payload()).unwrap();
            let decision = if request["command"] == "echo hi" {
                McpDecision::Deny
            } else {
                McpDecision::Allow
            };
            let sender = handle
                .state::<McpState>()
                .pending
                .lock()
                .unwrap()
                .remove(request["id"].as_str().unwrap());
            let _ = sender.unwrap().send(decision);
        });

        let untrusted = rpc(&server, "tools/call", call("true")).await;
        assert_eq!(untrusted["result"]["isError"], true);
        assert!(untrusted["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not trusted"));

        server
            .app
            .state::<SshTrustStore>()
            .confirm(crate::trust::HostTrustConfirmation {
                host: "127.0.0.1".to_string(),
                port: port as u16,
                algorithm: env("MCP_E2E_ALGORITHM"),
                fingerprint: env("MCP_E2E_FINGERPRINT"),
            })
            .await
            .unwrap();

        let denied = rpc(&server, "tools/call", call("echo hi")).await;
        assert!(denied["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("denied"));
        let ran = rpc(
            &server,
            "tools/call",
            call("echo out; echo err >&2; exit 3"),
        )
        .await;
        let text = ran["result"]["content"][0]["text"].as_str().unwrap();
        assert_eq!(ran["result"]["isError"], false, "{text}");
        assert!(
            text.contains("exit status: 3") && text.contains("out") && text.contains("err"),
            "{text}"
        );
    }

    #[test]
    fn tokens_compare_whole() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
