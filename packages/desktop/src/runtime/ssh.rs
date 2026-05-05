use russh::client::{self, AuthResult, Handle, Msg};
use russh::keys::ssh_key::HashAlg;
use russh::keys::ssh_key::PublicKey;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, Disconnect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use crate::trust::{HostTrustMismatch, HostTrustPrompt, SshTrustStore, TrustCheck};

use super::keys::{is_rsa_key, load_key_pair, rsa_hash_candidates};

const SSH_PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const SSH_PROBE_OUTPUT_LIMIT: usize = 16 * 1024;

const HOST_INFO_PROBE_COMMAND: &str = r#"sh -lc '
hostname_value=$(hostname 2>/dev/null || uname -n 2>/dev/null || true)
os_value=$(uname -s 2>/dev/null || true)
cpu_usage_percent_value=""
memory_total_bytes_value=""
memory_used_bytes_value=""
memory_usage_percent_value=""
if [ -r /proc/stat ] && [ -r /proc/meminfo ]; then
  cpu_line_1=$(awk "/^cpu / {print}" /proc/stat)
  sleep 1
  cpu_line_2=$(awk "/^cpu / {print}" /proc/stat)
  cpu_usage_percent_value=$(awk -v first="$cpu_line_1" -v second="$cpu_line_2" '\''
    BEGIN {
      split(first, a, " "); n = split(second, b, " ");
      idle1=a[5]+a[6]; idle2=b[5]+b[6];
      total1=0; total2=0;
      for (i=2; i<=n; i++) { total1 += a[i]; total2 += b[i]; }
      total_delta=total2-total1; idle_delta=idle2-idle1;
      if (total_delta > 0) printf "%.1f", (100 * (total_delta - idle_delta) / total_delta);
    }
  '\'')
  memory_total_kb=$(awk "/MemTotal/ {print \$2; exit}" /proc/meminfo)
  memory_available_kb=$(awk "/MemAvailable/ {print \$2; exit}" /proc/meminfo)
  memory_total_bytes_value=$(awk -v value="$memory_total_kb" "BEGIN {if (value > 0) printf \"%.0f\", value * 1024}")
  memory_used_bytes_value=$(awk -v total="$memory_total_kb" -v available="$memory_available_kb" "BEGIN {if (total > 0 && available >= 0) printf \"%.0f\", (total - available) * 1024}")
  memory_usage_percent_value=$(awk -v total="$memory_total_kb" -v available="$memory_available_kb" "BEGIN {if (total > 0 && available >= 0) printf \"%.1f\", 100 * (total - available) / total}")
elif command -v vm_stat >/dev/null 2>&1; then
  cpu_usage_percent_value=$(top -l 1 -n 0 2>/dev/null | awk -F"," "/CPU usage/ {gsub(/[^0-9.]/, \"\", \$1); print \$1; exit}")
  memory_total_bytes_value=$(sysctl -n hw.memsize 2>/dev/null || true)
  page_size=$(pagesize 2>/dev/null || echo 4096)
  pages_active=$(vm_stat 2>/dev/null | awk "/Pages active/ {gsub(/[^0-9]/, \"\", \$3); print \$3}")
  pages_wired=$(vm_stat 2>/dev/null | awk "/Pages wired down/ {gsub(/[^0-9]/, \"\", \$4); print \$4}")
  pages_compressed=$(vm_stat 2>/dev/null | awk "/Pages occupied by compressor/ {gsub(/[^0-9]/, \"\", \$5); print \$5}")
  memory_used_bytes_value=$(awk -v active="$pages_active" -v wired="$pages_wired" -v compressed="$pages_compressed" -v size="$page_size" "BEGIN {printf \"%.0f\", (active + wired + compressed) * size}")
  memory_usage_percent_value=$(awk -v used="$memory_used_bytes_value" -v total="$memory_total_bytes_value" "BEGIN {if (total > 0) printf \"%.1f\", 100 * used / total}")
fi
printf "hostname=%s\n" "$hostname_value"
printf "os=%s\n" "$os_value"
printf "cpu_usage_percent=%s\n" "$cpu_usage_percent_value"
printf "memory_total_bytes=%s\n" "$memory_total_bytes_value"
printf "memory_used_bytes=%s\n" "$memory_used_bytes_value"
printf "memory_usage_percent=%s\n" "$memory_usage_percent_value"
'"#;

pub(crate) struct ClientHandler {
    host: String,
    port: u16,
    trust_store: SshTrustStore,
    trust_check: Arc<Mutex<Option<TrustCheck>>>,
}

impl ClientHandler {
    fn new(
        host: String,
        port: u16,
        trust_store: SshTrustStore,
        trust_check: Arc<Mutex<Option<TrustCheck>>>,
    ) -> Self {
        Self {
            host,
            port,
            trust_store,
            trust_check,
        }
    }
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> impl std::future::Future<Output = Result<bool, Self::Error>> + Send {
        let host = self.host.clone();
        let port = self.port;
        let trust_store = self.trust_store.clone();
        let trust_check = self.trust_check.clone();
        let algorithm = server_public_key.algorithm().to_string();
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();

        async move {
            let check = trust_store
                .evaluate(&host, port, &algorithm, &fingerprint)
                .await;
            let trusted = matches!(check, TrustCheck::Trusted);
            *trust_check.lock().await = Some(check);
            Ok(trusted)
        }
    }
}

pub struct SshSession {
    pub(crate) handle: Arc<Mutex<Handle<ClientHandler>>>,
    pub(crate) channel_write: ChannelWriteHalf<Msg>,
    port_forwards: HashMap<String, SshPortForwardTask>,
}

struct SshPortForwardTask {
    status: SshPortForwardStatus,
    task: JoinHandle<()>,
}

#[derive(Default)]
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

        let config = client::Config {
            inactivity_timeout: None,
            ..<_>::default()
        };
        let config = Arc::new(config);
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

        let (channel_read, channel_write) = channel.split();

        let sid = session_id.clone();
        let sessions = self.sessions.clone();

        tokio::spawn(async move {
            read_loop(channel_read, sid, app, sessions).await;
        });
        info!(session_id, "Spawned SSH read loop");

        self.sessions.lock().await.insert(
            session_id.clone(),
            SshSession {
                handle: Arc::new(Mutex::new(session)),
                channel_write,
                port_forwards: HashMap::new(),
            },
        );

        info!(session_id, "SSH session established");
        Ok(SshConnectResponse::Connected { session_id })
    }

    pub async fn probe_host_info(
        &self,
        trust_store: SshTrustStore,
        host: String,
        port: u16,
        user: String,
        auth: AuthMethod,
    ) -> Result<SshProbeHostInfoResponse, String> {
        let probe_id = Uuid::new_v4().to_string();
        info!(probe_id, host, port, user, "Starting SSH host info probe");

        let config = client::Config {
            inactivity_timeout: Some(SSH_PROBE_TIMEOUT),
            ..<_>::default()
        };
        let config = Arc::new(config);
        let trust_check = Arc::new(Mutex::new(None));
        let handler = ClientHandler::new(host.clone(), port, trust_store, trust_check.clone());

        let connect_result = timeout(
            SSH_PROBE_TIMEOUT,
            client::connect(config, (host.clone(), port), handler),
        )
        .await
        .map_err(|_| "SSH probe timed out while connecting".to_string())?;

        let mut session = match connect_result {
            Ok(session) => session,
            Err(error) => return map_probe_connect_error(error, trust_check).await,
        };

        authenticate_session(&mut session, &user, auth, &probe_id).await?;

        let mut channel = timeout(SSH_PROBE_TIMEOUT, session.channel_open_session())
            .await
            .map_err(|_| "SSH probe timed out while opening channel".to_string())?
            .map_err(|error| format!("Failed to open probe channel: {error}"))?;

        timeout(
            SSH_PROBE_TIMEOUT,
            channel.exec(true, HOST_INFO_PROBE_COMMAND.to_string()),
        )
        .await
        .map_err(|_| "SSH probe timed out while starting command".to_string())?
        .map_err(|error| format!("Failed to start host info probe: {error}"))?;

        let output = collect_probe_output(&mut channel).await?;
        let _ = channel.close().await;

        Ok(SshProbeHostInfoResponse::Success {
            info: parse_host_system_info(&output),
        })
    }

    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        session
            .channel_write
            .data(&data[..])
            .await
            .map_err(|e| format!("Failed to write: {}", e))?;

        info!(
            session_id,
            bytes = data.len(),
            "Sent SSH input to remote channel"
        );

        Ok(())
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        session
            .channel_write
            .window_change(cols, rows, 0, 0)
            .await
            .map_err(|e| format!("Failed to resize: {}", e))?;

        info!(
            session_id,
            cols, rows, "Sent terminal resize to remote channel"
        );

        Ok(())
    }

    pub async fn start_local_port_forward(
        &self,
        app: AppHandle,
        input: SshLocalPortForwardInput,
    ) -> Result<SshPortForwardStatus, String> {
        let input = normalize_port_forward_input(input)?;

        let listener = TcpListener::bind((input.bind_host.as_str(), input.bind_port))
            .await
            .map_err(|e| {
                format!(
                    "Failed to bind local forward on {}:{}: {}",
                    input.bind_host, input.bind_port, e
                )
            })?;
        let bind_port = listener
            .local_addr()
            .map_err(|e| format!("Failed to inspect local forward listener: {e}"))?
            .port();

        let forward_id = Uuid::new_v4().to_string();
        let status = SshPortForwardStatus {
            forward_id: forward_id.clone(),
            session_id: input.session_id.clone(),
            bind_host: input.bind_host,
            bind_port,
            target_host: input.target_host,
            target_port: input.target_port,
            status: SshPortForwardState::Listening,
            error: None,
        };

        let handle = {
            let sessions = self.sessions.lock().await;
            sessions
                .get(&status.session_id)
                .ok_or_else(|| format!("Session not found: {}", status.session_id))?
                .handle
                .clone()
        };

        let task_status = status.clone();
        let task_sessions = self.sessions.clone();
        let task_app = app.clone();
        let task = tokio::spawn(async move {
            run_local_port_forward(listener, handle, task_status, task_app, task_sessions).await;
        });

        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&status.session_id) else {
            task.abort();
            return Err(format!("Session not found: {}", status.session_id));
        };
        session.port_forwards.insert(
            forward_id,
            SshPortForwardTask {
                status: status.clone(),
                task,
            },
        );

        emit_port_forward_event(&app, &status);
        info!(
            session_id = %status.session_id,
            forward_id = %status.forward_id,
            bind_host = %status.bind_host,
            bind_port = status.bind_port,
            target_host = %status.target_host,
            target_port = status.target_port,
            "Started SSH local port forward"
        );
        Ok(status)
    }

    pub async fn stop_port_forward(
        &self,
        app: AppHandle,
        session_id: &str,
        forward_id: &str,
    ) -> Result<SshPortForwardStatus, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        let forward = session
            .port_forwards
            .remove(forward_id)
            .ok_or_else(|| format!("Port forward not found: {forward_id}"))?;

        forward.task.abort();
        let status = stopped_port_forward_status(forward.status, None);
        emit_port_forward_event(&app, &status);
        info!(session_id, forward_id, "Stopped SSH local port forward");
        Ok(status)
    }

    pub async fn disconnect(&self, app: AppHandle, session_id: &str) -> Result<(), String> {
        let session = self.sessions.lock().await.remove(session_id);
        if let Some(mut session) = session {
            info!(session_id, "Disconnecting SSH session");
            let stopped_forwards = session.stop_port_forwards(None);
            let _ = session.channel_write.close().await;
            let _ = session
                .handle
                .lock()
                .await
                .disconnect(Disconnect::ByApplication, "Client disconnected", "")
                .await;
            for status in stopped_forwards {
                emit_port_forward_event(&app, &status);
            }
        }
        Ok(())
    }
}

impl SshSession {
    fn stop_port_forwards(&mut self, error: Option<String>) -> Vec<SshPortForwardStatus> {
        self.port_forwards
            .drain()
            .map(|(_, forward)| {
                forward.task.abort();
                stopped_port_forward_status(forward.status, error.clone())
            })
            .collect()
    }
}

enum PublicKeyAuthOutcome {
    Success,
    PartialSuccess,
    Rejected,
}

async fn authenticate_session(
    session: &mut Handle<ClientHandler>,
    user: &str,
    auth: AuthMethod,
    session_id: &str,
) -> Result<(), String> {
    match auth {
        AuthMethod::Password(password) => {
            info!(session_id, user, "Authenticating with password");
            let auth_res = session
                .authenticate_password(user.to_string(), password)
                .await
                .map_err(|error| format!("Password authentication failed: {error}"))?;
            if !auth_res.success() {
                return Err("Password authentication rejected".to_string());
            }
            info!(session_id, user, "Password authentication succeeded");
        }
        AuthMethod::PublicKey {
            private_key,
            passphrase,
        } => {
            info!(session_id, user, "Authenticating with key material");
            let key = load_key_pair(&private_key, passphrase.as_deref())?;
            match authenticate_public_key(session, user, key, session_id).await? {
                PublicKeyAuthOutcome::Success => {
                    info!(session_id, user, "Key authentication succeeded");
                }
                PublicKeyAuthOutcome::PartialSuccess => {
                    return Err(
                        "Key authentication accepted but requires additional authentication"
                            .to_string(),
                    );
                }
                PublicKeyAuthOutcome::Rejected => {
                    return Err("Key authentication rejected".to_string());
                }
            }
        }
        AuthMethod::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        } => {
            info!(
                session_id,
                user, "Authenticating with key + password material"
            );
            let key = load_key_pair(&private_key, passphrase.as_deref())?;
            match authenticate_public_key(session, user, key, session_id).await? {
                PublicKeyAuthOutcome::Success => {
                    info!(session_id, user, "Key authentication succeeded");
                }
                PublicKeyAuthOutcome::PartialSuccess | PublicKeyAuthOutcome::Rejected => {
                    info!(session_id, user, "Trying password after key authentication");
                    let auth_res = session
                        .authenticate_password(user.to_string(), password)
                        .await
                        .map_err(|error| format!("Password authentication failed: {error}"))?;
                    if !auth_res.success() {
                        return Err("Key + password authentication rejected".to_string());
                    }
                    info!(session_id, user, "Key + password authentication succeeded");
                }
            }
        }
    }

    Ok(())
}

async fn authenticate_public_key(
    session: &mut Handle<ClientHandler>,
    user: &str,
    key: russh::keys::PrivateKey,
    session_id: &str,
) -> Result<PublicKeyAuthOutcome, String> {
    let hash_candidates = if is_rsa_key(&key) {
        let server_best = session
            .best_supported_rsa_hash()
            .await
            .map_err(|error| format!("Failed to get supported RSA hash: {error}"))?;
        rsa_hash_candidates(server_best)
    } else {
        vec![None]
    };
    let key = Arc::new(key);

    for hash_alg in hash_candidates {
        info!(
            session_id,
            user,
            rsa_hash_alg = ?hash_alg,
            "Trying public key authentication"
        );
        let auth_res = session
            .authenticate_publickey(
                user.to_string(),
                russh::keys::PrivateKeyWithHashAlg::new(key.clone(), hash_alg),
            )
            .await
            .map_err(|error| format!("Key authentication failed: {error}"))?;
        match auth_res {
            AuthResult::Success => return Ok(PublicKeyAuthOutcome::Success),
            AuthResult::Failure {
                partial_success: true,
                ..
            } => return Ok(PublicKeyAuthOutcome::PartialSuccess),
            AuthResult::Failure { .. } => {}
        }
    }

    Ok(PublicKeyAuthOutcome::Rejected)
}

async fn read_loop(
    mut channel_read: ChannelReadHalf,
    session_id: String,
    app: AppHandle,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
) {
    loop {
        match channel_read.wait().await {
            Some(msg) => match msg {
                ChannelMsg::Data { ref data } => {
                    info!(session_id, bytes = data.len(), "Received SSH output chunk");
                    let output = String::from_utf8_lossy(data).to_string();
                    let event = SshOutputEvent {
                        session_id: session_id.clone(),
                        output,
                        closed: false,
                    };
                    if let Err(e) = app.emit("ssh_output", event) {
                        warn!(session_id, "Failed to emit ssh_output: {}", e);
                    }
                }
                ChannelMsg::Eof => {
                    info!(session_id, "Channel EOF received");
                    let event = SshOutputEvent {
                        session_id: session_id.clone(),
                        output: String::new(),
                        closed: true,
                    };
                    let _ = app.emit("ssh_output", event);
                    remove_session_and_stop_port_forwards(
                        sessions.clone(),
                        &session_id,
                        &app,
                        Some("SSH session closed".to_string()),
                    )
                    .await;
                    break;
                }
                ChannelMsg::Close => {
                    info!(session_id, "Channel closed");
                    let event = SshOutputEvent {
                        session_id: session_id.clone(),
                        output: String::new(),
                        closed: true,
                    };
                    let _ = app.emit("ssh_output", event);
                    remove_session_and_stop_port_forwards(
                        sessions.clone(),
                        &session_id,
                        &app,
                        Some("SSH session closed".to_string()),
                    )
                    .await;
                    break;
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    info!(session_id, "Exit status: {}", exit_status);
                }
                ChannelMsg::Success => {
                    info!(session_id, "Received SSH channel success message");
                }
                ChannelMsg::Failure => {
                    warn!(session_id, "Received SSH channel failure message");
                }
                _ => {}
            },
            None => {
                info!(session_id, "Channel stream ended");
                let event = SshOutputEvent {
                    session_id: session_id.clone(),
                    output: String::new(),
                    closed: true,
                };
                let _ = app.emit("ssh_output", event);
                remove_session_and_stop_port_forwards(
                    sessions.clone(),
                    &session_id,
                    &app,
                    Some("SSH session ended".to_string()),
                )
                .await;
                break;
            }
        }
    }
}

async fn remove_session_and_stop_port_forwards(
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    session_id: &str,
    app: &AppHandle,
    error: Option<String>,
) {
    let stopped_forwards = sessions
        .lock()
        .await
        .remove(session_id)
        .map(|mut session| session.stop_port_forwards(error))
        .unwrap_or_default();

    for status in stopped_forwards {
        emit_port_forward_event(app, &status);
    }
}

async fn run_local_port_forward(
    listener: TcpListener,
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    status: SshPortForwardStatus,
    app: AppHandle,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
) {
    let mut connections = JoinSet::new();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((local_stream, originator_addr)) => {
                        let connection_handle = handle.clone();
                        let connection_status = status.clone();
                        connections.spawn(async move {
                            handle_forward_connection(
                                local_stream,
                                originator_addr,
                                connection_handle,
                                connection_status,
                            ).await
                        });
                    }
                    Err(error) => {
                        let error_message = format!("Local forward listener failed: {error}");
                        warn!(
                            session_id = %status.session_id,
                            forward_id = %status.forward_id,
                            "{}",
                            error_message
                        );
                        emit_port_forward_event(&app, &error_port_forward_status(status.clone(), error_message));
                        break;
                    }
                }
            }
            Some(connection_result) = connections.join_next(), if !connections.is_empty() => {
                match connection_result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        warn!(
                            session_id = %status.session_id,
                            forward_id = %status.forward_id,
                            "SSH local port forward connection failed: {}",
                            error
                        );
                        emit_port_forward_event(&app, &error_port_forward_status(status.clone(), error));
                    }
                    Err(error) => {
                        let error_message = format!("Local forward connection task failed: {error}");
                        warn!(
                            session_id = %status.session_id,
                            forward_id = %status.forward_id,
                            "{}",
                            error_message
                        );
                        emit_port_forward_event(&app, &error_port_forward_status(status.clone(), error_message));
                    }
                }
            }
        }
    }

    connections.abort_all();
    let _ = sessions
        .lock()
        .await
        .get_mut(&status.session_id)
        .map(|session| session.port_forwards.remove(&status.forward_id));
}

async fn handle_forward_connection(
    mut local_stream: TcpStream,
    originator_addr: SocketAddr,
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    status: SshPortForwardStatus,
) -> Result<(), String> {
    let channel = handle
        .lock()
        .await
        .channel_open_direct_tcpip(
            status.target_host.clone(),
            u32::from(status.target_port),
            originator_addr.ip().to_string(),
            u32::from(originator_addr.port()),
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to open direct-tcpip channel to {}:{}: {}",
                status.target_host, status.target_port, e
            )
        })?;
    let mut remote_stream = channel.into_stream();
    copy_bidirectional(&mut local_stream, &mut remote_stream)
        .await
        .map_err(|e| format!("Failed to pipe local forward traffic: {e}"))?;
    Ok(())
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

fn normalize_port_forward_input(
    input: SshLocalPortForwardInput,
) -> Result<SshLocalPortForwardInput, String> {
    let session_id = input.session_id.trim().to_string();
    let bind_host = input.bind_host.trim().to_string();
    let target_host = input.target_host.trim().to_string();

    if input.session_id.trim().is_empty() {
        return Err("session_id is required".to_string());
    }
    if bind_host.is_empty() {
        return Err("bind_host is required".to_string());
    }
    if !is_loopback_bind_host(&bind_host) {
        return Err("Local port forwards can only bind to loopback hosts".to_string());
    }
    if target_host.is_empty() {
        return Err("target_host is required".to_string());
    }
    if input.target_port == 0 {
        return Err("target_port must be greater than 0".to_string());
    }

    Ok(SshLocalPortForwardInput {
        session_id,
        bind_host,
        bind_port: input.bind_port,
        target_host,
        target_port: input.target_port,
    })
}

fn is_loopback_bind_host(bind_host: &str) -> bool {
    matches!(bind_host, "127.0.0.1" | "localhost" | "::1")
}

fn stopped_port_forward_status(
    mut status: SshPortForwardStatus,
    error: Option<String>,
) -> SshPortForwardStatus {
    status.status = SshPortForwardState::Stopped;
    status.error = error;
    status
}

fn error_port_forward_status(
    mut status: SshPortForwardStatus,
    error: String,
) -> SshPortForwardStatus {
    status.status = SshPortForwardState::Error;
    status.error = Some(error);
    status
}

fn emit_port_forward_event(app: &AppHandle, status: &SshPortForwardStatus) {
    if let Err(e) = app.emit("ssh_port_forward", status.clone()) {
        warn!(
            session_id = %status.session_id,
            forward_id = %status.forward_id,
            "Failed to emit ssh_port_forward: {}",
            e
        );
    }
}

async fn collect_probe_output(channel: &mut russh::Channel<Msg>) -> Result<String, String> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_status: Option<u32> = None;

    let collect = async {
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => {
                    if stdout.len() + data.len() > SSH_PROBE_OUTPUT_LIMIT {
                        return Err("SSH probe output exceeded limit".to_string());
                    }
                    stdout.extend_from_slice(&data);
                }
                ChannelMsg::ExtendedData { data, .. } => {
                    if stderr.len() + data.len() > SSH_PROBE_OUTPUT_LIMIT {
                        return Err("SSH probe error output exceeded limit".to_string());
                    }
                    stderr.extend_from_slice(&data);
                }
                ChannelMsg::ExitStatus {
                    exit_status: status,
                } => {
                    exit_status = Some(status);
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }

        Ok::<(), String>(())
    };

    timeout(SSH_PROBE_TIMEOUT, collect)
        .await
        .map_err(|_| "SSH probe timed out while reading output".to_string())??;

    if !matches!(exit_status, None | Some(0)) {
        let stderr = String::from_utf8_lossy(&stderr).trim().to_string();
        if stderr.is_empty() {
            return Err(format!(
                "SSH probe exited with status {}",
                exit_status.unwrap_or(1)
            ));
        }
        return Err(format!(
            "SSH probe exited with status {}: {stderr}",
            exit_status.unwrap_or(1)
        ));
    }

    Ok(String::from_utf8_lossy(&stdout).to_string())
}

fn parse_host_system_info(output: &str) -> HostSystemInfo {
    let value = |key: &str| {
        output.lines().find_map(|line| {
            let (candidate_key, candidate_value) = line.split_once('=')?;
            if candidate_key == key {
                let value = candidate_value.trim();
                if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                }
            } else {
                None
            }
        })
    };

    HostSystemInfo {
        hostname: value("hostname"),
        os: value("os"),
        cpu_usage_percent: value("cpu_usage_percent").and_then(|value| value.parse::<f64>().ok()),
        memory_total_bytes: value("memory_total_bytes").and_then(|value| value.parse::<f64>().ok()),
        memory_used_bytes: value("memory_used_bytes").and_then(|value| value.parse::<f64>().ok()),
        memory_usage_percent: value("memory_usage_percent")
            .and_then(|value| value.parse::<f64>().ok()),
    }
}

async fn map_probe_connect_error(
    error: russh::Error,
    trust_check: Arc<Mutex<Option<TrustCheck>>>,
) -> Result<SshProbeHostInfoResponse, String> {
    match trust_check.lock().await.clone() {
        Some(TrustCheck::TrustRequired(prompt)) => {
            Ok(SshProbeHostInfoResponse::TrustRequired { prompt })
        }
        Some(TrustCheck::TrustMismatch(mismatch)) => {
            Ok(SshProbeHostInfoResponse::TrustMismatch { mismatch })
        }
        Some(TrustCheck::Trusted) | None => Err(format!("Failed to connect: {error}")),
    }
}

async fn map_connect_error(
    error: russh::Error,
    trust_check: Arc<Mutex<Option<TrustCheck>>>,
) -> Result<SshConnectResponse, String> {
    match trust_check.lock().await.clone() {
        Some(TrustCheck::TrustRequired(prompt)) => Ok(SshConnectResponse::TrustRequired { prompt }),
        Some(TrustCheck::TrustMismatch(mismatch)) => {
            Ok(SshConnectResponse::TrustMismatch { mismatch })
        }
        Some(TrustCheck::Trusted) | None => Err(format!("Failed to connect: {error}")),
    }
}

#[derive(Clone, Serialize)]
pub struct SshOutputEvent {
    pub session_id: String,
    pub output: String,
    pub closed: bool,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use crate::trust::{HostTrustMismatch, HostTrustPrompt, TrustCheck};

    use super::{map_connect_error, SshConnectResponse, SshSessionManager};

    #[test]
    fn ssh_session_manager_is_constructible() {
        let _ = SshSessionManager::new();
    }

    #[tokio::test]
    async fn trust_required_connect_error_surfaces_prompt() {
        let trust_check = Arc::new(Mutex::new(Some(TrustCheck::TrustRequired(
            HostTrustPrompt {
                host: "example.com".to_string(),
                port: 22,
                algorithm: "ssh-ed25519".to_string(),
                fingerprint: "SHA256:first".to_string(),
            },
        ))));

        let response = map_connect_error(russh::Error::UnknownKey, trust_check)
            .await
            .expect("trust prompt should be returned as a response");

        assert!(matches!(response, SshConnectResponse::TrustRequired { .. }));
    }

    #[tokio::test]
    async fn trust_mismatch_connect_error_surfaces_blocking_mismatch() {
        let trust_check = Arc::new(Mutex::new(Some(TrustCheck::TrustMismatch(
            HostTrustMismatch {
                host: "example.com".to_string(),
                port: 22,
                expected_algorithm: "ssh-ed25519".to_string(),
                expected_fingerprint: "SHA256:expected".to_string(),
                presented_algorithm: "ssh-ed25519".to_string(),
                presented_fingerprint: "SHA256:presented".to_string(),
            },
        ))));

        let response = map_connect_error(russh::Error::UnknownKey, trust_check)
            .await
            .expect("trust mismatch should be returned as a response");

        assert!(matches!(response, SshConnectResponse::TrustMismatch { .. }));
    }
}
