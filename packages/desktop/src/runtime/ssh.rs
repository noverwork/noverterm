mod authentication;
mod forwarding;
mod probe;
mod sftp;
mod shell;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use russh::Disconnect;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use crate::trust::{HostTrustMismatch, HostTrustPrompt, SshTrustStore};

use super::sftp::SftpSession;
use super::terminal_output::{OutputSession, TerminalOutput};
use authentication::{authenticate_session, client_config, map_connect_error};
use forwarding::{emit_port_forward_event, stopped_port_forward_status};
use shell::{read_loop, run_shell_write_loop, SSH_WRITE_QUEUE_CAPACITY};

pub(crate) use authentication::ClientHandler;

const SSH_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
const SSH_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(5);

pub struct SshSession {
    pub(crate) handle: Arc<Mutex<Handle<ClientHandler>>>,
    write_tx: Option<mpsc::Sender<SshWriteRequest>>,
    sftp_sessions: HashMap<String, Arc<SftpSession>>,
    port_forwards: HashMap<String, SshPortForwardTask>,
    keepalive_task: Option<JoinHandle<()>>,
    writer_task: Option<JoinHandle<()>>,
    output: Option<Arc<OutputSession>>,
}

struct SshWriteRequest {
    operation: SshWriteOperation,
    completion: oneshot::Sender<Result<(), String>>,
}

enum SshWriteOperation {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

struct SshPortForwardTask {
    status: SshPortForwardStatus,
    task: JoinHandle<()>,
}

#[derive(Clone, Default)]
pub struct SshSessionManager {
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
}

pub struct SshConnectRequest {
    pub app: AppHandle,
    pub trust_store: SshTrustStore,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: AuthMethod,
    pub cols: u32,
    pub rows: u32,
}

impl SshSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(&self, request: SshConnectRequest) -> Result<SshConnectResponse, String> {
        let SshConnectRequest {
            app,
            trust_store,
            host,
            port,
            user,
            auth,
            cols,
            rows,
        } = request;
        let session_id = Uuid::new_v4().to_string();
        info!(session_id, host, port, user, "Starting SSH connection flow");

        let config = client_config(None);
        let trust_check = Arc::new(Mutex::new(None));
        let handler = ClientHandler::new(host.clone(), port, trust_store, trust_check.clone());

        info!(session_id, host, port, "Opening TCP/SSH transport");
        let mut session = match client::connect(config, (host.clone(), port), handler).await {
            Ok(session) => session,
            Err(error) => return map_connect_error(error, trust_check).await,
        };
        info!(session_id, "TCP/SSH transport established");

        authenticate_session(&mut session, &user, auth, &session_id).await?;

        info!(session_id, "Opening SSH session channel");
        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| format!("Failed to open channel: {}", e))?;
        info!(session_id, "SSH session channel opened");

        info!(session_id, cols, rows, "Requesting PTY");
        channel
            .request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .map_err(|e| format!("Failed to request PTY: {}", e))?;
        info!(session_id, "PTY request accepted");

        info!(session_id, "Requesting interactive shell");
        channel
            .request_shell(true)
            .await
            .map_err(|e| format!("Failed to start shell: {}", e))?;
        info!(session_id, "Interactive shell started");

        let output = app.state::<TerminalOutput>().open(session_id.clone())?;
        let (channel_read, channel_write) = channel.split();
        let (write_tx, write_rx) = mpsc::channel(SSH_WRITE_QUEUE_CAPACITY);
        // Hold the map until insertion so an immediately exiting shell cannot
        // remove its entry before it exists.
        let mut sessions_guard = self.sessions.lock().await;
        let sid = session_id.clone();
        let sessions = self.sessions.clone();
        let read_app = app.clone();
        let read_output = output.clone();
        tokio::spawn(async move {
            read_loop(channel_read, sid, read_app, sessions, read_output).await;
        });
        let writer_task = tokio::spawn(run_shell_write_loop(
            channel_write,
            write_rx,
            output.clone(),
        ));

        let session_handle = Arc::new(Mutex::new(session));
        let keepalive_handle = session_handle.clone();
        let keepalive_sessions = self.sessions.clone();
        let keepalive_app = app.clone();
        let ka_session_id = session_id.clone();
        let keepalive_output = output.clone();
        let keepalive_task = tokio::spawn(async move {
            keepalive_loop(
                keepalive_handle,
                ka_session_id,
                keepalive_app,
                keepalive_sessions,
                Some(keepalive_output),
            )
            .await;
        });

        sessions_guard.insert(
            session_id.clone(),
            SshSession {
                handle: session_handle,
                write_tx: Some(write_tx),
                sftp_sessions: HashMap::new(),
                port_forwards: HashMap::new(),
                keepalive_task: Some(keepalive_task),
                writer_task: Some(writer_task),
                output: Some(output),
            },
        );

        info!(session_id, "SSH session established");
        Ok(SshConnectResponse::Connected { session_id })
    }

    pub async fn disconnect(&self, app: AppHandle, session_id: &str) -> Result<(), String> {
        let session = self.sessions.lock().await.remove(session_id);
        if let Some(mut session) = session {
            info!(session_id, "Disconnecting SSH session");
            let stopped_forwards = session.stop_runtime_tasks(None, true, true);
            let result = timeout(SSH_KEEPALIVE_TIMEOUT, async {
                session
                    .handle
                    .lock()
                    .await
                    .disconnect(Disconnect::ByApplication, "Client disconnected", "")
                    .await
            })
            .await
            .map_err(|_| "SSH disconnect timed out".to_string())
            .and_then(|result| result.map_err(|e| format!("SSH disconnect failed: {e}")));
            if let Some(output) = &session.output {
                if let Err(error) = &result {
                    output.fail(error.clone());
                }
                output.stop();
            }
            for status in stopped_forwards {
                emit_port_forward_event(&app, &status);
            }
            result?;
        }
        Ok(())
    }

    pub async fn contains_session(&self, session_id: &str) -> bool {
        self.sessions.lock().await.contains_key(session_id)
    }
}

impl SshSession {
    fn stop_runtime_tasks(
        &mut self,
        error: Option<String>,
        abort_keepalive: bool,
        abort_writer: bool,
    ) -> Vec<SshPortForwardStatus> {
        if abort_keepalive {
            self.abort_keepalive_task();
        }
        if abort_writer {
            self.abort_writer_task();
        }

        self.port_forwards
            .drain()
            .map(|(_, forward)| {
                forward.task.abort();
                stopped_port_forward_status(forward.status, error.clone())
            })
            .collect()
    }

    fn abort_keepalive_task(&mut self) {
        if let Some(task) = self.keepalive_task.take() {
            task.abort();
        }
    }

    fn abort_writer_task(&mut self) {
        if let Some(task) = self.writer_task.take() {
            task.abort();
        }
    }
}

async fn keepalive_loop(
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    session_id: String,
    app: AppHandle,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    output: Option<Arc<OutputSession>>,
) {
    let mut interval = tokio::time::interval(SSH_KEEPALIVE_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let error = loop {
        interval.tick().await;
        match timeout(SSH_KEEPALIVE_TIMEOUT, async {
            handle.lock().await.send_ping().await
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => break format!("SSH keepalive failed: {error}"),
            Err(_) => break "SSH keepalive timed out".to_string(),
        }
    };
    warn!(session_id, %error);
    if let Some(output) = output {
        // Let the reader flush its current batch before emitting the error.
        output.fail(error);
    } else {
        remove_session_and_stop_port_forwards(sessions, &session_id, &app, Some(error), false)
            .await;
    }
}

async fn remove_session_and_stop_port_forwards(
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    session_id: &str,
    app: &AppHandle,
    error: Option<String>,
    abort_keepalive: bool,
) {
    let session = sessions.lock().await.remove(session_id);
    let stopped_forwards = if let Some(mut session) = session {
        let stopped = session.stop_runtime_tasks(error, abort_keepalive, true);
        let _ = timeout(SSH_KEEPALIVE_TIMEOUT, async {
            session
                .handle
                .lock()
                .await
                .disconnect(Disconnect::ByApplication, "Terminal closed", "")
                .await
        })
        .await;
        stopped
    } else {
        Vec::new()
    };
    for status in stopped_forwards {
        emit_port_forward_event(app, &status);
    }
}

#[derive(Debug)]
pub enum AuthMethod {
    Password(String),
    PublicKey {
        private_key: String,
        passphrase: Option<String>,
    },
    PublicKeyAndPassword {
        private_key: String,
        passphrase: Option<String>,
        password: String,
    },
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SshConnectResponse {
    Connected { session_id: String },
    TrustRequired { prompt: HostTrustPrompt },
    TrustMismatch { mismatch: HostTrustMismatch },
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct HostSystemInfo {
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub cpu_usage_percent: Option<f64>,
    pub memory_total_bytes: Option<f64>,
    pub memory_used_bytes: Option<f64>,
    pub memory_usage_percent: Option<f64>,
    pub disk_read_bytes_per_second: Option<f64>,
    pub disk_write_bytes_per_second: Option<f64>,
    pub network_rx_bytes_per_second: Option<f64>,
    pub network_tx_bytes_per_second: Option<f64>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SshProbeHostInfoResponse {
    Success { info: HostSystemInfo },
    TrustRequired { prompt: HostTrustPrompt },
    TrustMismatch { mismatch: HostTrustMismatch },
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
pub struct SshLocalPortForwardInput {
    pub session_id: String,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SshPortForwardState {
    Listening,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct SshPortForwardStatus {
    pub forward_id: String,
    pub session_id: String,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub status: SshPortForwardState,
    pub error: Option<String>,
}
