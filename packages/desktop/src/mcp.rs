//! An MCP server on 127.0.0.1 that lets AI clients (Claude Code and others)
//! work on saved hosts: run commands (also in the background), read and write
//! files, run snippets, open port forwards, and read open terminal tabs.
//! Credentials, jump hosts and host trust stay here; everything that touches
//! a host or a terminal waits for the user's approval in the app first.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use crate::app::AppHandle;
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
use shared::{AiAccess, SnippetRecord};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinSet;
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use crate::runtime::local::LOCAL_TERMINAL_LABEL;
use crate::runtime::port_forward::{
    PortForwardAuth, PortForwardCreateInput, PortForwardManager, PortForwardStatus,
};
use crate::runtime::sftp::{FileType, SftpError};
use crate::runtime::ssh::{
    auth_method, exec_command, open_sftp, spawn_command, transfer, AuthMethod, BackgroundCommand,
    ExecOutput, ExecTarget, RemoteSftp, SshHopInput,
};
use crate::runtime::terminal_output::TerminalOutput;
use crate::store::hosts::set_ai_access;
use crate::store::snippets::load_snippets;
use crate::store::{internal_error, run_db, DbPool};
use crate::trust::SshTrustStore;

const SETTING_KEY: &str = "noverterm-mcp";
const DEFAULT_PORT: u16 = 47321;
const KEYRING_ACCOUNT: &str = "mcp-token";
/// An unanswered approval request is denied after this long.
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_RUN_TIMEOUT_SECS: u64 = 60;
const MAX_RUN_TIMEOUT_SECS: u64 = 600;
const DEFAULT_TRANSFER_TIMEOUT_SECS: u64 = 600;
const MAX_TRANSFER_TIMEOUT_SECS: u64 = 3600;
const MAX_JUMP_HOPS: usize = 5;
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const SFTP_TIMEOUT: Duration = Duration::from_secs(60);
const READ_FILE_LIMIT: usize = 256 * 1024;
const WRITE_FILE_LIMIT: usize = 1024 * 1024;
const MAX_MULTI_HOSTS: usize = 20;
const MAX_BACKGROUND: usize = 10;
const DEFAULT_READ_WAIT_SECS: u64 = 5;
const MAX_READ_WAIT_SECS: u64 = 60;
const DEFAULT_TERMINAL_LINES: u64 = 200;
const MAX_TERMINAL_LINES: u64 = 2000;
const FORWARD_START_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct McpState {
    server: Mutex<Option<RunningServer>>,
    pending: StdMutex<HashMap<String, oneshot::Sender<McpDecision>>>,
    /// Commands started with `start_command`, by id.
    background: StdMutex<HashMap<String, Arc<Background>>>,
    /// Port forwards started over MCP; only these can be stopped over MCP.
    forwards: StdMutex<HashSet<String>>,
    /// Tool calls in flight (waiting for approval or running), by id.
    running: StdMutex<HashMap<String, McpRunningTool>>,
}

struct Background {
    host_id: String,
    host_name: String,
    target: String,
    command: String,
    started_at: String,
    process: BackgroundCommand,
}

struct RunningServer {
    port: u16,
    shutdown: oneshot::Sender<()>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct McpSettings {
    pub port: u16,
    /// Let MCP clients read local terminal tabs without asking.
    #[serde(default)]
    pub allow_local_terminal: bool,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            allow_local_terminal: false,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct McpStatus {
    pub settings: McpSettings,
    pub running: bool,
    pub url: String,
    /// The bearer token clients must send; present once the server started.
    pub token: Option<String>,
    /// Why the server is not running.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpDecision {
    Allow,
    /// Allow this and later commands on the same host until the app restarts.
    AllowHost,
    Deny,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpApprovalRequest {
    pub id: String,
    pub host_name: String,
    pub target: String,
    pub command: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpActivity {
    pub host_name: String,
    pub command: String,
    /// `started`, `denied`, `failed`, `finished`, or `stopped`.
    pub status: String,
    pub exit_status: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct McpBackgroundCommand {
    pub id: String,
    pub host_name: String,
    pub command: String,
    /// RFC 3339.
    pub started_at: String,
    /// Finished, but the agent has not read the end yet.
    pub finished: bool,
}

/// A tool call an MCP client is waiting on right now.
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpRunningTool {
    pub id: String,
    pub tool: String,
    pub host: Option<String>,
    /// The command, path, or other argument that says what it does.
    pub detail: Option<String>,
    /// RFC 3339.
    pub started_at: String,
}

/// What MCP clients currently hold open in the app.
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpSessions {
    pub running: Vec<McpRunningTool>,
    pub commands: Vec<McpBackgroundCommand>,
    pub forwards: Vec<PortForwardStatus>,
}

pub async fn mcp_status(app: AppHandle) -> Result<McpStatus, String> {
    status(&app, None).await
}

pub async fn mcp_configure(app: AppHandle, settings: McpSettings) -> Result<McpStatus, String> {
    let previous = load_settings(&app).await?;
    save_settings(&app, settings).await?;
    // A toggle alone must not drop the clients connected to the running server.
    let running = app.state::<McpState>().server.lock().await.is_some();
    let error = if running && previous.port == settings.port {
        None
    } else {
        apply(&app, settings).await.err()
    };
    status(&app, error).await
}

async fn save_settings(app: &AppHandle, settings: McpSettings) -> Result<(), String> {
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
    .await
}

/// Replaces the token and restarts the server; clients set up with the old one stop working.
pub async fn mcp_regenerate_token(app: AppHandle) -> Result<McpStatus, String> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {}
        Err(error) => return Err(format!("Could not remove the MCP token: {error}")),
    }
    let error = apply(&app, load_settings(&app).await?).await.err();
    status(&app, error).await
}

pub async fn mcp_respond(
    state: crate::app::State<'_, McpState>,
    request_id: String,
    decision: McpDecision,
) -> Result<(), String> {
    if let Some(sender) = state.pending.lock().unwrap().remove(&request_id) {
        let _ = sender.send(decision);
    }
    Ok(())
}

pub async fn mcp_sessions(app: AppHandle) -> Result<McpSessions, String> {
    let state = app.state::<McpState>();
    let mut commands: Vec<McpBackgroundCommand> = state
        .background
        .lock()
        .unwrap()
        .iter()
        .map(|(id, entry)| McpBackgroundCommand {
            id: id.clone(),
            host_name: entry.host_name.clone(),
            command: entry.command.clone(),
            started_at: entry.started_at.clone(),
            finished: entry.process.finished(),
        })
        .collect();
    commands.sort_by(|left, right| left.started_at.cmp(&right.started_at));
    let ids = state.forwards.lock().unwrap().clone();
    let mut running: Vec<McpRunningTool> =
        state.running.lock().unwrap().values().cloned().collect();
    running.sort_by(|left, right| left.started_at.cmp(&right.started_at));
    let forwards = app
        .state::<PortForwardManager>()
        .list()
        .await
        .into_iter()
        .filter(|forward| ids.contains(&forward.id))
        .collect();
    Ok(McpSessions {
        running,
        commands,
        forwards,
    })
}

pub fn mcp_stop_command(app: AppHandle, id: String) -> Result<(), String> {
    stop_command(&app, &id).map(|_| ())
}

/// Tells the UI that `McpSessions` changed.
fn notify_changed(app: &AppHandle) {
    let _ = app.emit("mcp_changed", ());
}

/// Starts the server at launch. It listens on loopback only and every
/// request needs the bearer token, so it always runs.
pub async fn start(app: AppHandle) {
    let settings = match load_settings(&app).await {
        Ok(settings) => settings,
        Err(error) => {
            warn!(error, "Could not read MCP settings");
            McpSettings::default()
        }
    };
    if let Err(error) = apply(&app, settings).await {
        warn!(error, "MCP server did not start");
    }
}

async fn status(app: &AppHandle, error: Option<String>) -> Result<McpStatus, String> {
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

async fn load_settings(app: &AppHandle) -> Result<McpSettings, String> {
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

/// Stops any running server, then starts one on `settings.port`.
async fn apply(app: &AppHandle, settings: McpSettings) -> Result<(), String> {
    let state = app.state::<McpState>();
    let mut server = state.server.lock().await;
    if let Some(running) = server.take() {
        let _ = running.shutdown.send(());
        info!(port = running.port, "MCP server stopped");
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

async fn start_server(app: &AppHandle, port: u16, token: String) -> Result<RunningServer, String> {
    // Loopback only: nothing off this machine can reach it.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|error| format!("Could not listen on port {port}: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let router = Router::new()
        .route("/mcp", post(handle).get(not_allowed).delete(not_allowed))
        .with_state(Arc::new(Context {
            app: app.clone(),
            token,
        }));
    let (shutdown, stopped) = oneshot::channel::<()>();
    crate::app::runtime().spawn(async move {
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

struct Context {
    app: AppHandle,
    token: String,
}

async fn not_allowed() -> StatusCode {
    // No server-initiated stream: clients fall back to plain POST replies.
    StatusCode::METHOD_NOT_ALLOWED
}

async fn handle(State(ctx): State<Arc<Context>>, headers: HeaderMap, body: Bytes) -> Response {
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
                    "instructions": "Work on the user's saved SSH hosts: run commands, read and write files, \
                        run saved snippets, open port forwards, and read open terminal tabs. \
                        Call list_hosts first. Everything that touches a host or terminal needs the user's approval \
                        in noverterm, so explain why you do it and prefer read-only actions.",
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
    let host = json!({ "type": "string", "description": "Host name from list_hosts." });
    let remote_path = json!({ "type": "string", "description": "Path on the host; relative paths start in the user's home." });
    let run_timeout = json!({ "type": "integer", "description": "Stop the command after this many seconds (default 60, max 600)." });
    let transfer_properties = json!({
        "host": host,
        "local_path": { "type": "string", "description": "Absolute path on this machine." },
        "remote_path": remote_path,
        "timeout_secs": { "type": "integer", "description": "Stop the transfer after this many seconds (default 600, max 3600)." },
    });
    let command_id = json!({ "type": "string", "description": "Id that start_command returned." });
    json!([
        tool("list_hosts", "List the SSH hosts saved in noverterm: name, user@host:port, and any jump hosts it connects through.",
            json!({}), &[], true),
        tool("run_command", "Run a non-interactive shell command on a saved host and return its exit status, stdout and stderr. \
                No TTY: avoid commands that prompt for input; use start_command for those and for long-running ones.",
            json!({ "host": host, "command": { "type": "string", "description": "Shell command to run." }, "timeout_secs": run_timeout }),
            &["host", "command"], false),
        tool("run_command_multi", "Run the same non-interactive shell command on several saved hosts at once. \
                The user approves it on each host; results come back per host.",
            json!({
                "hosts": { "type": "array", "items": { "type": "string" }, "description": "Host names from list_hosts (max 20)." },
                "command": { "type": "string", "description": "Shell command to run." },
                "timeout_secs": run_timeout,
            }),
            &["hosts", "command"], false),
        tool("upload_file", "Copy a local file or directory to a saved host over SFTP. Overwrites the remote path.",
            transfer_properties.clone(), &["host", "local_path", "remote_path"], false),
        tool("download_file", "Copy a file or directory from a saved host to this machine over SFTP. Overwrites the local path.",
            transfer_properties, &["host", "local_path", "remote_path"], false),
        tool("list_dir", "List a directory on a saved host over SFTP: type, size and name of each entry.",
            json!({ "host": host, "path": { "type": "string", "description": "Directory on the host (default: the user's home)." } }),
            &["host"], true),
        tool("read_file", "Read a text file on a saved host over SFTP (first 256 KiB). Use download_file for binary or larger files.",
            json!({ "host": host, "path": remote_path }), &["host", "path"], true),
        tool("write_file", "Create or overwrite a text file on a saved host over SFTP with the given content (max 1 MiB).",
            json!({ "host": host, "path": remote_path, "content": { "type": "string", "description": "The whole new file content." } }),
            &["host", "path", "content"], false),
        tool("list_snippets", "List the command snippets the user saved in noverterm, with the host each belongs to.",
            json!({}), &[], true),
        tool("run_snippet", "Run a saved snippet as a non-interactive command, on its own host unless host is given.",
            json!({
                "snippet": { "type": "string", "description": "Snippet title from list_snippets." },
                "host": { "type": "string", "description": "Host to run it on; needed for local terminal snippets." },
                "timeout_secs": run_timeout,
            }),
            &["snippet"], false),
        tool("list_port_forwards", "List the port forwards running in noverterm, including ones the user started.",
            json!({}), &[], true),
        tool("start_port_forward", "Forward a port on 127.0.0.1 of this machine to target_host:target_port as seen from a saved host, \
                e.g. to reach a database that only listens on the host. Returns the local port and the forward id.",
            json!({
                "host": host,
                "target_port": { "type": "integer", "description": "Port to reach from the host." },
                "target_host": { "type": "string", "description": "Address to reach from the host (default 127.0.0.1)." },
                "local_port": { "type": "integer", "description": "Local port to listen on (default: any free port)." },
            }),
            &["host", "target_port"], false),
        tool("stop_port_forward", "Stop a port forward that start_port_forward opened.",
            json!({ "id": { "type": "string", "description": "Forward id from start_port_forward." } }), &["id"], false),
        tool("start_command", "Start a long-running or interactive command on a saved host and return its id at once. \
                Follow with read_command for output, write_command for input, and stop_command when done.",
            json!({
                "host": host,
                "command": { "type": "string", "description": "Shell command to run." },
                "tty": { "type": "boolean", "description": "Give the command a terminal, for programs that need one such as sudo prompts (default false)." },
            }),
            &["host", "command"], false),
        tool("read_command", "Read the output a started command printed since the last read, and whether it finished.",
            json!({
                "id": command_id,
                "wait_secs": { "type": "integer", "description": "Wait up to this many seconds for new output (default 5, max 60)." },
            }),
            &["id"], true),
        tool("write_command", "Send input to a started command's stdin. Include a trailing newline to submit a line.",
            json!({
                "id": command_id,
                "input": { "type": "string", "description": "Text to send." },
                "close_stdin": { "type": "boolean", "description": "Close stdin after sending (default false)." },
            }),
            &["id"], false),
        tool("stop_command", "Stop a started command and forget it.", json!({ "id": command_id }), &["id"], false),
        tool("list_terminals", "List the terminal tabs open in noverterm: id and what each is connected to.",
            json!({}), &[], true),
        tool("read_terminal", "Read the recent output of an open terminal tab as plain text, to see what the user sees.",
            json!({
                "terminal": { "type": "string", "description": "Terminal id from list_terminals." },
                "lines": { "type": "integer", "description": "How many of the last lines to return (default 200, max 2000)." },
            }),
            &["terminal"], true),
    ])
}

fn tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
    read_only: bool,
) -> Value {
    let (description, annotations) = if read_only {
        (description.to_string(), json!({ "readOnlyHint": true }))
    } else {
        (
            format!("{description} The user approves it in noverterm first."),
            json!({ "destructiveHint": true, "openWorldHint": true }),
        )
    };
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
        "annotations": annotations,
    })
}

/// Lists the call in `McpState.running` while it runs, so the status line
/// shows the agent at work even for one-shot tools.
async fn call_tool(app: &AppHandle, params: &Value) -> Result<String, String> {
    struct InFlight<'a>(&'a AppHandle, String);
    // Also runs when the client hangs up and the call is dropped mid-way.
    impl Drop for InFlight<'_> {
        fn drop(&mut self) {
            self.0
                .state::<McpState>()
                .running
                .lock()
                .unwrap()
                .remove(&self.1);
            notify_changed(self.0);
        }
    }

    let args = &params["arguments"];
    let host = optional(args, "host").map(str::to_string).or_else(|| {
        let hosts: Vec<&str> = args["hosts"]
            .as_array()?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        (!hosts.is_empty()).then(|| hosts.join(", "))
    });
    let detail = [
        "command",
        "path",
        "remote_path",
        "snippet",
        "terminal",
        "id",
    ]
    .into_iter()
    .find_map(|name| optional(args, name))
    .map(str::to_string);
    let id = Uuid::new_v4().to_string();
    app.state::<McpState>().running.lock().unwrap().insert(
        id.clone(),
        McpRunningTool {
            id: id.clone(),
            tool: params["name"].as_str().unwrap_or_default().to_string(),
            host,
            detail,
            started_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    notify_changed(app);
    let _in_flight = InFlight(app, id);
    dispatch_tool(app, params).await
}

async fn dispatch_tool(app: &AppHandle, params: &Value) -> Result<String, String> {
    let args = &params["arguments"];
    let run_timeout = || {
        seconds(
            args,
            "timeout_secs",
            DEFAULT_RUN_TIMEOUT_SECS,
            MAX_RUN_TIMEOUT_SECS,
        )
    };
    match params["name"].as_str().unwrap_or_default() {
        "list_hosts" => list_hosts(app).await,
        "run_command" => {
            let (host, command) = (required(args, "host")?, required(args, "command")?);
            run_command(app, host, command, run_timeout()).await
        }
        "run_command_multi" => {
            let hosts: Vec<String> = args["hosts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|host| host.as_str().map(|host| host.trim().to_string()))
                .filter(|host| !host.is_empty())
                .collect();
            if hosts.is_empty() || hosts.len() > MAX_MULTI_HOSTS {
                return Err(format!("hosts must list 1 to {MAX_MULTI_HOSTS} host names"));
            }
            run_command_multi(app, hosts, required(args, "command")?, run_timeout()).await
        }
        name @ ("upload_file" | "download_file") => {
            let (host, local, remote) = (
                required(args, "host")?,
                required(args, "local_path")?,
                required(args, "remote_path")?,
            );
            if !std::path::Path::new(local).is_absolute() {
                return Err("local_path must be absolute".to_string());
            }
            let transfer_timeout = seconds(
                args,
                "timeout_secs",
                DEFAULT_TRANSFER_TIMEOUT_SECS,
                MAX_TRANSFER_TIMEOUT_SECS,
            );
            transfer_file(
                app,
                host,
                name == "upload_file",
                local,
                remote,
                transfer_timeout,
            )
            .await
        }
        "list_dir" => {
            let path = optional(args, "path").unwrap_or(".");
            list_dir(app, required(args, "host")?, path).await
        }
        "read_file" => read_file(app, required(args, "host")?, required(args, "path")?).await,
        "write_file" => {
            let content = args["content"].as_str().ok_or("content is required")?;
            write_file(
                app,
                required(args, "host")?,
                required(args, "path")?,
                content,
            )
            .await
        }
        "list_snippets" => list_snippets(app).await,
        "run_snippet" => {
            let snippet = required(args, "snippet")?;
            run_snippet(app, snippet, optional(args, "host"), run_timeout()).await
        }
        "list_port_forwards" => list_port_forwards(app).await,
        "start_port_forward" => {
            let target_port = port_arg(args, "target_port")?.ok_or("target_port is required")?;
            let target_host = optional(args, "target_host").unwrap_or("127.0.0.1");
            let local_port = port_arg(args, "local_port")?.unwrap_or(0);
            let host = required(args, "host")?;
            start_port_forward(app, host, local_port, target_host, target_port).await
        }
        "stop_port_forward" => stop_port_forward(app, required(args, "id")?).await,
        "start_command" => {
            let (host, command) = (required(args, "host")?, required(args, "command")?);
            start_command(app, host, command, args["tty"].as_bool().unwrap_or(false)).await
        }
        "read_command" => {
            let wait = number(
                args,
                "wait_secs",
                DEFAULT_READ_WAIT_SECS,
                MAX_READ_WAIT_SECS,
            );
            read_command(app, required(args, "id")?, Duration::from_secs(wait)).await
        }
        "write_command" => {
            let input = args["input"].as_str().unwrap_or_default();
            let close_stdin = args["close_stdin"].as_bool().unwrap_or(false);
            write_command(app, required(args, "id")?, input, close_stdin).await
        }
        "stop_command" => stop_command(app, required(args, "id")?),
        "list_terminals" => list_terminals(app),
        "read_terminal" => {
            let lines = number(args, "lines", DEFAULT_TERMINAL_LINES, MAX_TERMINAL_LINES);
            read_terminal(app, required(args, "terminal")?, lines as usize).await
        }
        name => Err(format!("Unknown tool: {name}")),
    }
}

fn optional<'a>(args: &'a Value, name: &str) -> Option<&'a str> {
    args[name]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn required<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    optional(args, name).ok_or_else(|| format!("{name} is required"))
}

fn number(args: &Value, name: &str, default: u64, max: u64) -> u64 {
    args[name].as_u64().unwrap_or(default).min(max)
}

fn seconds(args: &Value, name: &str, default: u64, max: u64) -> Duration {
    Duration::from_secs(number(args, name, default, max).max(1))
}

fn port_arg(args: &Value, name: &str) -> Result<Option<u16>, String> {
    match &args[name] {
        Value::Null => Ok(None),
        value => value
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("{name} must be a port number")),
    }
}

async fn list_hosts(app: &AppHandle) -> Result<String, String> {
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
                "target": host_target(host),
                "via": via,
                "ai_access": host.ai_access,
            })
        })
        .collect();
    serde_json::to_string_pretty(&listed).map_err(|error| error.to_string())
}

async fn run_command(
    app: &AppHandle,
    host_ref: &str,
    command: &str,
    run_timeout: Duration,
) -> Result<String, String> {
    let (host, target) = approved_target(app, host_ref, command).await?;
    let host_name = host.name;
    let trust_store = app.state::<SshTrustStore>().inner().clone();
    let result = exec_command(&trust_store, target, command, run_timeout).await;
    match &result {
        Ok(output) => emit_activity(app, &host_name, command, "finished", output.exit_status),
        Err(_) => emit_activity(app, &host_name, command, "failed", None),
    }
    result.map(|output| format_output(&output))
}

async fn transfer_file(
    app: &AppHandle,
    host_ref: &str,
    upload: bool,
    local_path: &str,
    remote_path: &str,
    run_timeout: Duration,
) -> Result<String, String> {
    // Shown in the approval dialog and activity log in place of a command.
    let summary = if upload {
        format!("upload {local_path} -> {remote_path}")
    } else {
        format!("download {remote_path} -> {local_path}")
    };
    let (host, target) = approved_target(app, host_ref, &summary).await?;
    let host_name = host.name;
    let trust_store = app.state::<SshTrustStore>().inner().clone();
    let result = transfer(
        &trust_store,
        target,
        upload,
        local_path,
        remote_path,
        run_timeout,
    )
    .await;
    match &result {
        Ok(_) => emit_activity(app, &host_name, &summary, "finished", None),
        Err(_) => emit_activity(app, &host_name, &summary, "failed", None),
    }
    result.map(|bytes| format!("Copied {bytes} bytes ({summary})"))
}

/// Resolves a saved host into a connect target once the user approves `action` on it.
async fn approved_target(
    app: &AppHandle,
    host_ref: &str,
    action: &str,
) -> Result<(SshHost, ExecTarget), String> {
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

    approve(
        app,
        AiAccess::parse(&host.ai_access),
        Remember::Host(host.id.clone()),
        &host.name,
        &host_target(host),
        action,
    )
    .await?;
    let target = ExecTarget {
        host: host.host.clone(),
        port: port(host)?,
        username: host.username.clone(),
        auth,
        jumps,
    };
    Ok((host.clone(), target))
}

const DENIED: &str = "The user denied this in noverterm.";

fn host_target(host: &SshHost) -> String {
    format!("{}@{}:{}", host.username, host.host, host.port)
}

fn emit_activity(
    app: &AppHandle,
    host_name: &str,
    command: &str,
    status: &str,
    exit_status: Option<u32>,
) {
    let _ = app.emit(
        "mcp_activity",
        McpActivity {
            host_name: host_name.to_string(),
            command: command.to_string(),
            status: status.to_string(),
            exit_status,
        },
    );
}

/// What "Always allow" in the approval dialog turns on.
enum Remember {
    Host(String),
    LocalTerminal,
    /// A terminal on an unsaved (or ambiguous) host: allow this call only.
    Nothing,
}

/// Lets the call through per `access`, asking the user when it is `Ask`.
async fn approve(
    app: &AppHandle,
    access: AiAccess,
    remember: Remember,
    host_name: &str,
    target: &str,
    command: &str,
) -> Result<(), String> {
    match access {
        AiAccess::Allow => return Ok(()),
        AiAccess::Deny => {
            emit_activity(app, host_name, command, "denied", None);
            return Err(format!(
                "The user blocked AI access to {host_name} in noverterm."
            ));
        }
        AiAccess::Ask => {}
    }
    let state = app.state::<McpState>();
    let id = Uuid::new_v4().to_string();
    let (sender, receiver) = oneshot::channel();
    state.pending.lock().unwrap().insert(id.clone(), sender);
    let request = McpApprovalRequest {
        id: id.clone(),
        host_name: host_name.to_string(),
        target: target.to_string(),
        command: command.to_string(),
    };
    let sent = app.emit("mcp_approval_request", request).is_ok();
    let decision = if sent {
        tokio::time::timeout(APPROVAL_TIMEOUT, receiver).await.ok()
    } else {
        None
    };
    state.pending.lock().unwrap().remove(&id);
    match decision {
        Some(Ok(McpDecision::Allow)) => Ok(()),
        Some(Ok(McpDecision::AllowHost)) => {
            // The call still runs if saving fails; the user did allow it.
            let saved = match remember {
                Remember::Host(host_id) => {
                    let pool = app.state::<DbPool>().inner().clone();
                    set_ai_access(pool, host_id, AiAccess::Allow).await
                }
                Remember::LocalTerminal => match load_settings(app).await {
                    Ok(settings) => {
                        save_settings(
                            app,
                            McpSettings {
                                allow_local_terminal: true,
                                ..settings
                            },
                        )
                        .await
                    }
                    Err(error) => Err(error),
                },
                Remember::Nothing => Ok(()),
            };
            match saved {
                Ok(()) => {
                    let _ = app.emit("ai_access_changed", ());
                }
                Err(error) => warn!(error, "Could not save AI access"),
            }
            Ok(())
        }
        _ => {
            emit_activity(app, host_name, command, "denied", None);
            Err(DENIED.to_string())
        }
    }
}

/// A host's current AI access; a host deleted since asks.
async fn host_access(app: &AppHandle, host_id: &str) -> AiAccess {
    let pool = app.state::<DbPool>().inner().clone();
    let host_id = host_id.to_string();
    run_db(pool, move |db| {
        ssh_hosts::table
            .filter(ssh_hosts::id.eq(host_id))
            .select(ssh_hosts::ai_access)
            .first::<String>(db)
            .map_err(internal_error)
    })
    .await
    .map_or(AiAccess::Ask, |access| AiAccess::parse(&access))
}

/// A terminal tab follows the saved host it is connected to, matched by its
/// `user@host:port` label; a local tab follows the MCP setting. Saved hosts
/// sharing one target take the strictest access.
// ponytail: matches by label, so a direct (unsaved) connection always asks; pass the host id through ssh_connect_direct if that matters.
async fn terminal_access(app: &AppHandle, label: &str) -> Result<(AiAccess, Remember), String> {
    if label == LOCAL_TERMINAL_LABEL {
        let access = if load_settings(app).await?.allow_local_terminal {
            AiAccess::Allow
        } else {
            AiAccess::Ask
        };
        return Ok((access, Remember::LocalTerminal));
    }
    let (hosts, _) = load_hosts(app).await?;
    let matching: Vec<&SshHost> = hosts
        .iter()
        .filter(|host| host_target(host) == label)
        .collect();
    let access = matching
        .iter()
        .map(|host| AiAccess::parse(&host.ai_access))
        .max_by_key(|access| match access {
            AiAccess::Allow => 0,
            AiAccess::Ask => 1,
            AiAccess::Deny => 2,
        })
        .unwrap_or(AiAccess::Ask);
    let remember = match matching.as_slice() {
        [host] => Remember::Host(host.id.clone()),
        _ => Remember::Nothing,
    };
    Ok((access, remember))
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

async fn run_command_multi(
    app: &AppHandle,
    hosts: Vec<String>,
    command: &str,
    run_timeout: Duration,
) -> Result<String, String> {
    let mut runs = JoinSet::new();
    for (index, host) in hosts.into_iter().enumerate() {
        let (app, command) = (app.clone(), command.to_string());
        runs.spawn(async move {
            let result = run_command(&app, &host, &command, run_timeout).await;
            (index, host, result)
        });
    }
    let mut results = runs.join_all().await;
    results.sort_by_key(|(index, ..)| *index);
    Ok(results
        .into_iter()
        .map(|(_, host, result)| match result {
            Ok(text) => format!("=== {host} ===\n{text}"),
            Err(error) => format!("=== {host} ===\nerror: {error}\n"),
        })
        .collect())
}

/// Opens SFTP on a host once the user approves `action`.
async fn approved_sftp(
    app: &AppHandle,
    host_ref: &str,
    action: &str,
) -> Result<(String, RemoteSftp), String> {
    let (host, target) = approved_target(app, host_ref, action).await?;
    let trust_store = app.state::<SshTrustStore>().inner().clone();
    match open_sftp(&trust_store, target).await {
        Ok(remote) => Ok((host.name, remote)),
        Err(error) => {
            emit_activity(app, &host.name, action, "failed", None);
            Err(error)
        }
    }
}

/// Logs how an SFTP call bounded by `SFTP_TIMEOUT` went.
fn sftp_outcome<T>(
    app: &AppHandle,
    host_name: &str,
    action: &str,
    result: Result<Result<T, SftpError>, tokio::time::error::Elapsed>,
) -> Result<T, String> {
    let result = match result {
        Ok(result) => result.map_err(|error| error.to_string()),
        Err(_) => Err(format!("Stopped after {} seconds", SFTP_TIMEOUT.as_secs())),
    };
    let status = if result.is_ok() { "finished" } else { "failed" };
    emit_activity(app, host_name, action, status, None);
    result
}

async fn list_dir(app: &AppHandle, host_ref: &str, path: &str) -> Result<String, String> {
    let action = format!("list {path}");
    let (host_name, remote) = approved_sftp(app, host_ref, &action).await?;
    let result = timeout(SFTP_TIMEOUT, remote.sftp.list_dir(path)).await;
    remote.close().await;
    let entries = sftp_outcome(app, &host_name, &action, result)?;
    Ok(entries
        .iter()
        .map(|entry| {
            let kind = match entry.file_type {
                FileType::Dir => 'd',
                FileType::Symlink => 'l',
                _ => '-',
            };
            format!("{kind} {:>12} {}\n", entry.size, entry.name)
        })
        .collect())
}

async fn read_file(app: &AppHandle, host_ref: &str, path: &str) -> Result<String, String> {
    let action = format!("read {path}");
    let (host_name, remote) = approved_sftp(app, host_ref, &action).await?;
    let result = timeout(SFTP_TIMEOUT, remote.sftp.read_file(path, READ_FILE_LIMIT)).await;
    remote.close().await;
    let (data, truncated) = sftp_outcome(app, &host_name, &action, result)?;
    if data.contains(&0) {
        return Err(format!("{path} is not a text file; use download_file"));
    }
    let mut text = String::from_utf8_lossy(&data).into_owned();
    if truncated {
        text.push_str("\n(truncated at 256 KiB; use download_file for the whole file)");
    }
    Ok(text)
}

async fn write_file(
    app: &AppHandle,
    host_ref: &str,
    path: &str,
    content: &str,
) -> Result<String, String> {
    if content.len() > WRITE_FILE_LIMIT {
        return Err("content is larger than 1 MiB; use upload_file".to_string());
    }
    // The user reviews the content itself, not only the path.
    let preview: String = content.chars().take(2000).collect();
    let more = if preview.len() < content.len() {
        "\n…"
    } else {
        ""
    };
    let action = format!("write {path} ({} bytes)\n{preview}{more}", content.len());
    let (host_name, remote) = approved_sftp(app, host_ref, &action).await?;
    let result = timeout(
        SFTP_TIMEOUT,
        remote.sftp.write_file(path, content.as_bytes()),
    )
    .await;
    remote.close().await;
    sftp_outcome(app, &host_name, &action, result)?;
    Ok(format!("Wrote {} bytes to {path}", content.len()))
}

async fn list_snippets(app: &AppHandle) -> Result<String, String> {
    let snippets = load_snippets(app.state::<DbPool>().inner().clone()).await?;
    let listed: Vec<Value> = snippets
        .iter()
        .map(|snippet| {
            json!({
                "title": snippet.title,
                "command": snippet.body,
                // Local terminal snippets have no host.
                "host": snippet.host_id.as_ref().map(|_| &snippet.host_name),
            })
        })
        .collect();
    serde_json::to_string_pretty(&listed).map_err(|error| error.to_string())
}

async fn run_snippet(
    app: &AppHandle,
    snippet_ref: &str,
    host_ref: Option<&str>,
    run_timeout: Duration,
) -> Result<String, String> {
    let snippets = load_snippets(app.state::<DbPool>().inner().clone()).await?;
    let snippet = find_snippet(&snippets, snippet_ref)?;
    let host = host_ref
        .or(snippet.host_id.as_deref())
        .ok_or_else(|| format!("{} is a local terminal snippet; pass a host", snippet.title))?;
    run_command(app, host, &snippet.body, run_timeout).await
}

fn find_snippet<'a>(
    snippets: &'a [SnippetRecord],
    snippet_ref: &str,
) -> Result<&'a SnippetRecord, String> {
    if let Some(snippet) = snippets.iter().find(|snippet| snippet.id == snippet_ref) {
        return Ok(snippet);
    }
    let matches: Vec<&SnippetRecord> = snippets
        .iter()
        .filter(|snippet| snippet.title.eq_ignore_ascii_case(snippet_ref))
        .collect();
    match matches.as_slice() {
        [snippet] => Ok(snippet),
        [] => Err(format!(
            "No snippet titled {snippet_ref}. Call list_snippets to see them."
        )),
        _ => Err(format!(
            "More than one snippet is titled {snippet_ref}; rename one in noverterm."
        )),
    }
}

async fn list_port_forwards(app: &AppHandle) -> Result<String, String> {
    let forwards = app.state::<PortForwardManager>().list().await;
    serde_json::to_string_pretty(&forwards).map_err(|error| error.to_string())
}

async fn start_port_forward(
    app: &AppHandle,
    host_ref: &str,
    local_port: u16,
    target_host: &str,
    target_port: u16,
) -> Result<String, String> {
    let local = if local_port == 0 {
        "any port".to_string()
    } else {
        local_port.to_string()
    };
    let action = format!("forward 127.0.0.1:{local} -> {target_host}:{target_port}");
    let (host, target) = approved_target(app, host_ref, &action).await?;
    let auth = match target.auth {
        AuthMethod::Password(password) => PortForwardAuth::Password(password),
        AuthMethod::PublicKey {
            private_key,
            passphrase,
        } => PortForwardAuth::PublicKey {
            private_key,
            passphrase,
        },
        AuthMethod::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        } => PortForwardAuth::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        },
    };

    // The forward connects in the background; its status events say how it went.
    let (events, mut statuses) = mpsc::unbounded_channel::<Value>();
    let listener = app.listen("port_forward_status", move |event| {
        if let Ok(status) = serde_json::from_str(event.payload()) {
            let _ = events.send(status);
        }
    });
    let trust_store = app.state::<SshTrustStore>().inner().clone();
    let started = app
        .state::<PortForwardManager>()
        .start(
            app.clone(),
            PortForwardCreateInput {
                name: format!("MCP: {}", host.name),
                host: target.host,
                port: target.port,
                username: target.username,
                auth,
                jumps: target.jumps,
                bind_host: "127.0.0.1".to_string(),
                bind_port: local_port,
                target_host: target_host.to_string(),
                target_port,
            },
            trust_store,
        )
        .await;
    let settled = match &started {
        Ok(started) => {
            timeout(FORWARD_START_TIMEOUT, async {
                while let Some(status) = statuses.recv().await {
                    if status["id"] == started.id.as_str() && status["state"] != "connecting" {
                        return Some(status);
                    }
                }
                None
            })
            .await
        }
        Err(_) => Ok(None),
    };
    app.unlisten(listener);
    let started =
        started.inspect_err(|_| emit_activity(app, &host.name, &action, "failed", None))?;

    match settled {
        Ok(Some(status)) if status["state"] == "listening" => {
            app.state::<McpState>()
                .forwards
                .lock()
                .unwrap()
                .insert(started.id.clone());
            notify_changed(app);
            emit_activity(app, &host.name, &action, "finished", None);
            Ok(format!(
                "Forwarding 127.0.0.1:{} to {target_host}:{target_port} through {}. Forward id: {}",
                status["bind_port"], host.name, started.id
            ))
        }
        Ok(Some(status)) => {
            emit_activity(app, &host.name, &action, "failed", None);
            Err(status["error"]
                .as_str()
                .unwrap_or("The port forward failed")
                .to_string())
        }
        _ => {
            app.state::<McpState>()
                .forwards
                .lock()
                .unwrap()
                .insert(started.id.clone());
            notify_changed(app);
            Ok(format!(
                "The port forward is still connecting; check list_port_forwards. Forward id: {}",
                started.id
            ))
        }
    }
}

async fn stop_port_forward(app: &AppHandle, id: &str) -> Result<String, String> {
    if !app.state::<McpState>().forwards.lock().unwrap().remove(id) {
        return Err(
            "Only port forwards started with start_port_forward can be stopped here".to_string(),
        );
    }
    let status = app
        .state::<PortForwardManager>()
        .stop(app.clone(), id)
        .await?;
    notify_changed(app);
    Ok(format!(
        "Stopped forwarding 127.0.0.1:{} to {}:{}",
        status.bind_port, status.target_host, status.target_port
    ))
}

async fn start_command(
    app: &AppHandle,
    host_ref: &str,
    command: &str,
    tty: bool,
) -> Result<String, String> {
    let state = app.state::<McpState>();
    if state.background.lock().unwrap().len() >= MAX_BACKGROUND {
        return Err(format!(
            "{MAX_BACKGROUND} commands are already running; stop one with stop_command"
        ));
    }
    let action = format!("start in background: {command}");
    let (host, target) = approved_target(app, host_ref, &action).await?;
    let trust_store = app.state::<SshTrustStore>().inner().clone();
    let process = spawn_command(&trust_store, target, command, tty)
        .await
        .inspect_err(|_| emit_activity(app, &host.name, &action, "failed", None))?;
    let id = Uuid::new_v4().to_string();
    state.background.lock().unwrap().insert(
        id.clone(),
        Arc::new(Background {
            target: host_target(&host),
            host_id: host.id,
            host_name: host.name.clone(),
            command: command.to_string(),
            process,
            started_at: chrono::Utc::now().to_rfc3339(),
        }),
    );
    notify_changed(app);
    emit_activity(app, &host.name, &action, "started", None);
    Ok(format!("Started. Command id: {id}"))
}

fn background(app: &AppHandle, id: &str) -> Result<Arc<Background>, String> {
    app.state::<McpState>()
        .background
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or_else(|| format!("No running command with id {id}"))
}

async fn read_command(app: &AppHandle, id: &str, wait: Duration) -> Result<String, String> {
    let entry = background(app, id)?;
    let read = entry.process.read(wait).await;
    let mut text = if read.finished {
        app.state::<McpState>()
            .background
            .lock()
            .unwrap()
            .remove(id);
        notify_changed(app);
        emit_activity(
            app,
            &entry.host_name,
            &entry.command,
            "finished",
            read.exit_status,
        );
        match read.exit_status {
            Some(status) => format!("finished, exit status: {status}\n"),
            None => "finished\n".to_string(),
        }
    } else {
        "running\n".to_string()
    };
    if read.dropped {
        text.push_str("(older output was dropped; read more often)\n");
    }
    if !read.output.is_empty() {
        text.push_str(&format!("--- output ---\n{}\n", read.output.trim_end()));
    }
    Ok(text)
}

async fn write_command(
    app: &AppHandle,
    id: &str,
    input: &str,
    close_stdin: bool,
) -> Result<String, String> {
    let entry = background(app, id)?;
    let close = if close_stdin {
        "\n(then close stdin)"
    } else {
        ""
    };
    let action = format!("input to {}:\n{input}{close}", entry.command);
    approve(
        app,
        host_access(app, &entry.host_id).await,
        Remember::Host(entry.host_id.clone()),
        &entry.host_name,
        &entry.target,
        &action,
    )
    .await?;
    if !input.is_empty() {
        entry.process.write(Some(input.as_bytes().to_vec()))?;
    }
    if close_stdin {
        entry.process.write(None)?;
    }
    Ok("Sent".to_string())
}

fn stop_command(app: &AppHandle, id: &str) -> Result<String, String> {
    // Dropping the last handle stops the command.
    let entry = app
        .state::<McpState>()
        .background
        .lock()
        .unwrap()
        .remove(id);
    let entry = entry.ok_or_else(|| format!("No running command with id {id}"))?;
    notify_changed(app);
    emit_activity(app, &entry.host_name, &entry.command, "stopped", None);
    Ok("Stopped".to_string())
}

fn list_terminals(app: &AppHandle) -> Result<String, String> {
    let listed: Vec<Value> = app
        .state::<TerminalOutput>()
        .terminals()
        .into_iter()
        .map(|(id, label)| json!({ "id": id, "target": label }))
        .collect();
    serde_json::to_string_pretty(&listed).map_err(|error| error.to_string())
}

async fn read_terminal(app: &AppHandle, id: &str, lines: usize) -> Result<String, String> {
    let missing = || "No open terminal with that id. Call list_terminals to see them.".to_string();
    let (label, _) = app
        .state::<TerminalOutput>()
        .history(id)
        .ok_or_else(missing)?;
    let action = "read the terminal's recent output";
    let (access, remember) = terminal_access(app, &label).await?;
    approve(app, access, remember, &label, &label, action).await?;
    // Read after approval so the user sees what is on screen now.
    let (_, bytes) = app
        .state::<TerminalOutput>()
        .history(id)
        .ok_or_else(missing)?;
    emit_activity(app, &label, action, "finished", None);
    let text = terminal_text(&bytes);
    let all: Vec<&str> = text.lines().collect();
    Ok(all[all.len().saturating_sub(lines)..].join("\n"))
}

/// Plain text of raw terminal output: escape sequences dropped, and carriage
/// returns and backspaces applied within a line.
// ponytail: no cursor-movement emulation, so full-screen apps (vim, top) come out garbled; feed a vt100 parser if that matters.
fn terminal_text(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    let mut chars = raw.chars().peekable();
    let (mut text, mut line) = (String::new(), String::new());
    while let Some(char) = chars.next() {
        match char {
            '\x1b' => match chars.next() {
                // CSI: parameters, then one final byte in @..~.
                Some('[') => {
                    for char in chars.by_ref() {
                        if ('@'..='~').contains(&char) {
                            break;
                        }
                    }
                }
                // OSC and other strings: up to BEL or ESC \.
                Some(']' | 'P' | 'X' | '^' | '_') => {
                    while let Some(char) = chars.next() {
                        if char == '\x07' {
                            break;
                        }
                        if char == '\x1b' {
                            chars.next();
                            break;
                        }
                    }
                }
                // Character set selection takes one more byte.
                Some('(' | ')' | '*' | '+' | '#' | '%') => {
                    chars.next();
                }
                _ => {}
            },
            '\n' => {
                text.push_str(line.trim_end());
                text.push('\n');
                line.clear();
            }
            '\r' if chars.peek() != Some(&'\n') => line.clear(),
            '\x08' => {
                line.pop();
            }
            '\t' => line.push('\t'),
            char if char.is_control() => {}
            char => line.push(char),
        }
    }
    text.push_str(line.trim_end());
    text
}

async fn load_hosts(app: &AppHandle) -> Result<(Vec<SshHost>, Vec<SshKey>), String> {
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
            ai_access: "ask".to_string(),
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
        app: AppHandle,
        url: String,
        _server: RunningServer,
        _directory: tempfile::TempDir,
    }

    async fn test_server() -> TestServer {
        let directory = tempfile::tempdir().unwrap();
        let app = AppHandle::default();
        let pool = crate::store::test_pool(&directory);
        app.manage(SshTrustStore::new(pool.clone()));
        app.manage(pool);
        app.manage(McpState::default());
        app.manage(TerminalOutput::default());
        app.manage(PortForwardManager::new());
        let server = start_server(&app, 0, "secret".to_string()).await.unwrap();
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
                ai_access: "ask".to_string(),
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
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 19);

        insert_host(&server, "web", "web.test", 22, "pw");
        let listed = rpc(&server, "tools/call", json!({ "name": "list_hosts" })).await;
        let text = listed["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("root@web.test:22"), "{text}");
        assert!(!text.contains("pw"), "credentials never leave: {text}");
    }

    #[tokio::test]
    async fn host_ai_access_blocks_asks_and_remembers_always_allow() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let server = test_server().await;
        // Nothing listens on port 1, so an allowed call fails only after approval.
        insert_host(&server, "web", "127.0.0.1", 1, "pw");
        let asked = Arc::new(AtomicUsize::new(0));
        let (handle, counter) = (server.app.clone(), asked.clone());
        server.app.listen("mcp_approval_request", move |event| {
            counter.fetch_add(1, Ordering::SeqCst);
            let request: Value = serde_json::from_str(event.payload()).unwrap();
            let sender = handle
                .state::<McpState>()
                .pending
                .lock()
                .unwrap()
                .remove(request["id"].as_str().unwrap());
            let _ = sender.unwrap().send(McpDecision::AllowHost);
        });
        let pool = || server.app.state::<DbPool>().inner().clone();
        let call =
            json!({ "name": "run_command", "arguments": { "host": "web", "command": "true" } });
        let text = |reply: &Value| {
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .to_string()
        };

        set_ai_access(pool(), "web".to_string(), AiAccess::Deny)
            .await
            .unwrap();
        let blocked = rpc(&server, "tools/call", call.clone()).await;
        assert!(text(&blocked).contains("blocked"), "{}", text(&blocked));
        assert_eq!(asked.load(Ordering::SeqCst), 0);

        set_ai_access(pool(), "web".to_string(), AiAccess::Ask)
            .await
            .unwrap();
        rpc(&server, "tools/call", call.clone()).await;
        assert_eq!(asked.load(Ordering::SeqCst), 1);
        assert_eq!(host_access(&server.app, "web").await, AiAccess::Allow);

        rpc(&server, "tools/call", call).await;
        assert_eq!(
            asked.load(Ordering::SeqCst),
            1,
            "allowed hosts are not asked again"
        );
    }

    /// Needs an sshd; see the env vars below. Run with `--ignored`.
    #[tokio::test]
    #[ignore]
    async fn runs_an_approved_command_over_ssh() {
        let env = |name: &str| std::env::var(name).unwrap();
        let server = test_server().await;
        let port: i32 = env("MCP_E2E_PORT").parse().unwrap();
        insert_host(&server, "box", "127.0.0.1", port, &env("MCP_E2E_PASSWORD"));
        let call = |command: &str| json!({ "name": "run_command", "arguments": { "host": "box", "command": command } });

        // Stand in for the user: deny `echo hi`, allow everything else.
        let handle = server.app.clone();
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
    fn terminal_text_drops_escapes_and_applies_overwrites() {
        let raw = b"\x1b]0;title\x07\x1b[1;32mok\x1b[0m done  \r\n\
            50%\r100%\nab\x08c\n\x1b(Bplain";
        assert_eq!(terminal_text(raw), "ok done\n100%\nac\nplain");
    }

    #[test]
    fn tokens_compare_whole() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
    }
}
