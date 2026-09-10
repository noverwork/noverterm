use russh::client::{self, AuthResult, Handle, Msg};
use russh::keys::ssh_key::HashAlg;
use russh::keys::ssh_key::PublicKey;
use russh::Preferred;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, Disconnect};
use russh_sftp::client::SftpSession as RusshSftpSession;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::timeout;
use tracing::{info, warn};
use uuid::Uuid;

use crate::trust::{HostTrustMismatch, HostTrustPrompt, SshTrustStore, TrustCheck};

use super::keys::{is_rsa_key, load_key_pair, rsa_hash_candidates};
use super::sftp::{
    open_sftp_session, sftp_client_config, FileEntry, SftpSession, TransferCancellation,
    TransferDirection, TransferProgress,
};
use super::terminal_output::{OutputSession, TerminalOutput, OUTPUT_CHUNK_BYTES};

const SSH_PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const SSH_PROBE_OUTPUT_LIMIT: usize = 16 * 1024;

const SSH_OUTPUT_FLUSH_INTERVAL: Duration = Duration::from_millis(2);
const SSH_WRITE_QUEUE_CAPACITY: usize = 16;
const SSH_WRITE_MAX_BYTES: usize = 1024 * 1024;

const SSH_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
const SSH_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(5);

fn client_config(inactivity_timeout: Option<Duration>) -> Arc<client::Config> {
    Arc::new(client::Config {
        inactivity_timeout,
        preferred: Preferred {
            compression: std::borrow::Cow::Borrowed(&[
                russh::compression::ZLIB,
                russh::compression::ZLIB_LEGACY,
                russh::compression::NONE,
            ]),
            ..Preferred::DEFAULT
        },
        ..<_>::default()
    })
}

const HOST_INFO_PROBE_COMMAND: &str = r#"sh -lc '
hostname_value=$(hostname 2>/dev/null || uname -n 2>/dev/null || true)
os_value=$(uname -s 2>/dev/null || true)
cpu_usage_percent_value=""
memory_total_bytes_value=""
memory_used_bytes_value=""
memory_usage_percent_value=""
disk_read_bytes_per_second_value=""
disk_write_bytes_per_second_value=""
network_rx_bytes_per_second_value=""
network_tx_bytes_per_second_value=""
if [ -r /proc/stat ] && [ -r /proc/meminfo ]; then
  cpu_line_1=$(awk "/^cpu / {print}" /proc/stat)
  disk_sample_1=""
  disk_sample_2=""
  network_sample_1=""
  network_sample_2=""
  if [ -r /proc/diskstats ]; then
    disk_sample_1=$(awk "{
      name=\$3;
      if (name ~ /^(loop|ram|zram)/) next;
      if (name ~ /^(sd[a-z]+[0-9]+|vd[a-z]+[0-9]+|xvd[a-z]+[0-9]+|nvme[0-9]+n[0-9]+p[0-9]+|mmcblk[0-9]+p[0-9]+)$/) next;
      read += \$6; write += \$10; found=1;
    }
    END { if (found) printf \"%.0f %.0f\", read * 512, write * 512 }" /proc/diskstats)
  fi
  if [ -r /proc/net/dev ]; then
    network_sample_1=$(awk -F"[: ]+" "NR > 2 {
      iface=\$2;
      if (iface != \"\" && iface != \"lo\") { rx += \$3; tx += \$11; found=1; }
    }
    END { if (found) printf \"%.0f %.0f\", rx, tx }" /proc/net/dev)
  fi
  sleep 1
  cpu_line_2=$(awk "/^cpu / {print}" /proc/stat)
  if [ -r /proc/diskstats ]; then
    disk_sample_2=$(awk "{
      name=\$3;
      if (name ~ /^(loop|ram|zram)/) next;
      if (name ~ /^(sd[a-z]+[0-9]+|vd[a-z]+[0-9]+|xvd[a-z]+[0-9]+|nvme[0-9]+n[0-9]+p[0-9]+|mmcblk[0-9]+p[0-9]+)$/) next;
      read += \$6; write += \$10; found=1;
    }
    END { if (found) printf \"%.0f %.0f\", read * 512, write * 512 }" /proc/diskstats)
  fi
  if [ -r /proc/net/dev ]; then
    network_sample_2=$(awk -F"[: ]+" "NR > 2 {
      iface=\$2;
      if (iface != \"\" && iface != \"lo\") { rx += \$3; tx += \$11; found=1; }
    }
    END { if (found) printf \"%.0f %.0f\", rx, tx }" /proc/net/dev)
  fi
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
  disk_values=$(awk -v first="$disk_sample_1" -v second="$disk_sample_2" "BEGIN {
    split(first, a, \" \"  ); split(second, b, \" \"  );
    if (a[1] != \"\" && b[1] != \"\" && b[1] >= a[1]) printf \"%.0f\", b[1] - a[1];
    printf \" \";
    if (a[2] != \"\" && b[2] != \"\" && b[2] >= a[2]) printf \"%.0f\", b[2] - a[2];
  }")
  disk_read_bytes_per_second_value=${disk_values%% *}
  disk_write_bytes_per_second_value=${disk_values#* }
  if [ "$disk_read_bytes_per_second_value" = "$disk_values" ]; then disk_write_bytes_per_second_value=""; fi
  network_values=$(awk -v first="$network_sample_1" -v second="$network_sample_2" "BEGIN {
    split(first, a, \" \"  ); split(second, b, \" \"  );
    if (a[1] != \"\" && b[1] != \"\" && b[1] >= a[1]) printf \"%.0f\", b[1] - a[1];
    printf \" \";
    if (a[2] != \"\" && b[2] != \"\" && b[2] >= a[2]) printf \"%.0f\", b[2] - a[2];
  }")
  network_rx_bytes_per_second_value=${network_values%% *}
  network_tx_bytes_per_second_value=${network_values#* }
  if [ "$network_rx_bytes_per_second_value" = "$network_values" ]; then network_tx_bytes_per_second_value=""; fi
elif command -v vm_stat >/dev/null 2>&1; then
  cpu_usage_percent_value=$(top -l 1 -n 0 2>/dev/null | awk -F"," "/CPU usage/ {gsub(/[^0-9.]/, \"\", \$1); print \$1; exit}")
  memory_total_bytes_value=$(sysctl -n hw.memsize 2>/dev/null || true)
  page_size=$(pagesize 2>/dev/null || echo 4096)
  pages_active=$(vm_stat 2>/dev/null | awk "/Pages active/ {gsub(/[^0-9]/, \"\", \$3); print \$3}")
  pages_wired=$(vm_stat 2>/dev/null | awk "/Pages wired down/ {gsub(/[^0-9]/, \"\", \$4); print \$4}")
  pages_compressed=$(vm_stat 2>/dev/null | awk "/Pages occupied by compressor/ {gsub(/[^0-9]/, \"\", \$5); print \$5}")
  memory_used_bytes_value=$(awk -v active="$pages_active" -v wired="$pages_wired" -v compressed="$pages_compressed" -v size="$page_size" "BEGIN {printf \"%.0f\", (active + wired + compressed) * size}")
  memory_usage_percent_value=$(awk -v used="$memory_used_bytes_value" -v total="$memory_total_bytes_value" "BEGIN {if (total > 0) printf \"%.1f\", 100 * used / total}")
  if command -v netstat >/dev/null 2>&1; then
    network_sample_1=$(netstat -ibn 2>/dev/null | awk "
      NR == 1 {
        for (i=1; i<=NF; i++) {
          if (\$i == \"Ibytes\") rx_col=i;
          if (\$i == \"Obytes\") tx_col=i;
        }
        next;
      }
      rx_col > 0 && tx_col > 0 && \$1 !~ /^lo/ {
        if (\$rx_col ~ /^[0-9]+$/ && \$tx_col ~ /^[0-9]+$/) {
          rx += \$rx_col; tx += \$tx_col; found=1;
        }
      }
      END { if (found) printf \"%.0f %.0f\", rx, tx }")
    sleep 1
    network_sample_2=$(netstat -ibn 2>/dev/null | awk "
      NR == 1 {
        for (i=1; i<=NF; i++) {
          if (\$i == \"Ibytes\") rx_col=i;
          if (\$i == \"Obytes\") tx_col=i;
        }
        next;
      }
      rx_col > 0 && tx_col > 0 && \$1 !~ /^lo/ {
        if (\$rx_col ~ /^[0-9]+$/ && \$tx_col ~ /^[0-9]+$/) {
          rx += \$rx_col; tx += \$tx_col; found=1;
        }
      }
      END { if (found) printf \"%.0f %.0f\", rx, tx }")
    network_values=$(awk -v first="$network_sample_1" -v second="$network_sample_2" "BEGIN {
      split(first, a, \" \"  ); split(second, b, \" \"  );
      if (a[1] != \"\" && b[1] != \"\" && b[1] >= a[1]) printf \"%.0f\", b[1] - a[1];
      printf \" \";
      if (a[2] != \"\" && b[2] != \"\" && b[2] >= a[2]) printf \"%.0f\", b[2] - a[2];
    }")
    network_rx_bytes_per_second_value=${network_values%% *}
    network_tx_bytes_per_second_value=${network_values#* }
    if [ "$network_rx_bytes_per_second_value" = "$network_values" ]; then network_tx_bytes_per_second_value=""; fi
  fi
fi
printf "hostname=%s\n" "$hostname_value"
printf "os=%s\n" "$os_value"
printf "cpu_usage_percent=%s\n" "$cpu_usage_percent_value"
printf "memory_total_bytes=%s\n" "$memory_total_bytes_value"
printf "memory_used_bytes=%s\n" "$memory_used_bytes_value"
printf "memory_usage_percent=%s\n" "$memory_usage_percent_value"
printf "disk_read_bytes_per_second=%s\n" "$disk_read_bytes_per_second_value"
printf "disk_write_bytes_per_second=%s\n" "$disk_write_bytes_per_second_value"
printf "network_rx_bytes_per_second=%s\n" "$network_rx_bytes_per_second_value"
printf "network_tx_bytes_per_second=%s\n" "$network_tx_bytes_per_second_value"
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

        let config = client_config(Some(SSH_PROBE_TIMEOUT));
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
        if data.len() > SSH_WRITE_MAX_BYTES {
            return Err("SSH terminal input payload is too large".to_string());
        }
        self.shell_request(session_id, SshWriteOperation::Data(data))
            .await
    }

    async fn shell_request(
        &self,
        session_id: &str,
        operation: SshWriteOperation,
    ) -> Result<(), String> {
        let (completion, result) = oneshot::channel();
        {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| format!("Session not found: {session_id}"))?;
            let write_tx = session
                .write_tx
                .as_ref()
                .ok_or("No shell channel available for this session")?;
            write_tx
                .try_send(SshWriteRequest {
                    operation,
                    completion,
                })
                .map_err(map_ssh_write_queue_error)?;
        }
        result
            .await
            .map_err(|_| "SSH write loop stopped before completing input".to_string())?
    }

    pub async fn open_sftp(&self, session_id: &str) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;

        session.open_sftp().await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn connect_direct_sftp(
        &self,
        app: AppHandle,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        private_key: Option<&str>,
        passphrase: Option<&str>,
        trust_store: SshTrustStore,
    ) -> Result<String, String> {
        let sftp_session_id = Uuid::new_v4().to_string();
        info!(
            sftp_session_id,
            host, port, username, "Starting direct SFTP connection"
        );

        let config = client_config(None);
        let trust_check = Arc::new(Mutex::new(None));
        let handler = ClientHandler::new(host.to_string(), port, trust_store, trust_check.clone());

        let mut session = match client::connect(config, (host.to_string(), port), handler).await {
            Ok(session) => session,
            Err(error) => {
                let response = map_connect_error(error, trust_check).await;
                return match response {
                    Ok(SshConnectResponse::Connected { .. }) => {
                        Err("Unexpected connected response".to_string())
                    }
                    Ok(SshConnectResponse::TrustRequired { prompt }) => {
                        Err(format!("Trust required: {}", prompt.fingerprint))
                    }
                    Ok(SshConnectResponse::TrustMismatch { mismatch }) => {
                        Err(format!("Trust mismatch: {}", mismatch.expected_fingerprint))
                    }
                    Err(e) => Err(e),
                };
            }
        };

        let auth_method = if let Some(key_content) = private_key {
            AuthMethod::PublicKey {
                private_key: key_content.to_string(),
                passphrase: passphrase.map(|s| s.to_string()),
            }
        } else if let Some(pwd) = password {
            AuthMethod::Password(pwd.to_string())
        } else {
            return Err("No authentication method provided".to_string());
        };

        authenticate_session(&mut session, username, auth_method, &sftp_session_id).await?;

        info!(sftp_session_id, "SFTP authentication successful");

        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| format!("Failed to open channel: {}", e))?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("Failed to request SFTP subsystem: {}", e))?;

        info!(sftp_session_id, "SFTP subsystem requested");

        let sftp = RusshSftpSession::new_with_config(channel.into_stream(), sftp_client_config())
            .await
            .map_err(|e| format!("Failed to create SFTP session: {}", e))?;

        let sftp_session = SftpSession::new(sftp_session_id.clone(), sftp);

        let session_handle = Arc::new(Mutex::new(session));
        let keepalive_handle = session_handle.clone();
        let keepalive_sessions = self.sessions.clone();
        let keepalive_app = app.clone();
        let ka_session_id = sftp_session_id.clone();

        let keepalive_task = tokio::spawn(async move {
            keepalive_loop(
                keepalive_handle,
                ka_session_id,
                keepalive_app,
                keepalive_sessions,
                None,
            )
            .await;
        });

        self.sessions.lock().await.insert(
            sftp_session_id.clone(),
            SshSession {
                handle: session_handle,
                write_tx: None,
                sftp_sessions: HashMap::from([(sftp_session_id.clone(), Arc::new(sftp_session))]),
                port_forwards: HashMap::new(),
                keepalive_task: Some(keepalive_task),
                writer_task: None,
                output: None,
            },
        );

        info!(sftp_session_id, "Direct SFTP session established");
        Ok(sftp_session_id)
    }

    pub async fn close_sftp(&self, sftp_id: &str) -> Result<(), String> {
        let (ssh_id, sftp_session) = {
            let mut sessions = self.sessions.lock().await;
            remove_sftp_session(&mut sessions, sftp_id)?
        };

        if let Err(error) = sftp_session.close().await {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(&ssh_id) {
                session
                    .sftp_sessions
                    .insert(sftp_id.to_string(), sftp_session);
            }
            return Err(error.to_string());
        }

        Ok(())
    }

    pub async fn sftp_list_dir(&self, sftp_id: &str, path: &str) -> Result<Vec<FileEntry>, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .list_dir(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_home_dir(&self, sftp_id: &str) -> Result<String, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .home_dir()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_stat(&self, sftp_id: &str, path: &str) -> Result<FileEntry, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .stat(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_mkdir(&self, sftp_id: &str, path: &str) -> Result<(), String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .mkdir(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_remove(&self, sftp_id: &str, path: &str) -> Result<(), String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .remove(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_rename(&self, sftp_id: &str, old: &str, new: &str) -> Result<(), String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .rename(old, new)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_transfer_conflicts(
        &self,
        sftp_id: &str,
        direction: TransferDirection,
        source_path: &str,
        target_path: &str,
    ) -> Result<Vec<String>, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .transfer_conflicts(direction, source_path, target_path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_upload(
        &self,
        sftp_id: &str,
        local_path: &str,
        remote_path: &str,
        transfer_id: String,
        cancel: TransferCancellation,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<u64, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .upload(local_path, remote_path, transfer_id, cancel, progress_tx)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_download(
        &self,
        sftp_id: &str,
        remote_path: &str,
        local_path: &str,
        transfer_id: String,
        cancel: TransferCancellation,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<u64, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .download(remote_path, local_path, transfer_id, cancel, progress_tx)
            .await
            .map_err(|error| error.to_string())
    }

    async fn sftp_session(&self, sftp_id: &str) -> Result<Arc<SftpSession>, String> {
        let sessions = self.sessions.lock().await;
        find_sftp_session(&sessions, sftp_id)
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<(), String> {
        self.shell_request(session_id, SshWriteOperation::Resize { cols, rows })
            .await
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

    pub async fn contains_sftp_session(&self, sftp_id: &str) -> bool {
        self.sessions
            .lock()
            .await
            .values()
            .any(|session| session.sftp_sessions.contains_key(sftp_id))
    }
}

fn find_sftp_session(
    sessions: &HashMap<String, SshSession>,
    sftp_id: &str,
) -> Result<Arc<SftpSession>, String> {
    sessions
        .values()
        .find_map(|session| session.sftp_sessions.get(sftp_id).cloned())
        .ok_or_else(|| format!("SFTP session not found: {sftp_id}"))
}

fn remove_sftp_session(
    sessions: &mut HashMap<String, SshSession>,
    sftp_id: &str,
) -> Result<(String, Arc<SftpSession>), String> {
    for (ssh_id, session) in sessions.iter_mut() {
        if let Some(sftp_session) = session.sftp_sessions.remove(sftp_id) {
            return Ok((ssh_id.clone(), sftp_session));
        }
    }

    Err(format!("SFTP session not found: {sftp_id}"))
}

impl SshSession {
    pub async fn open_sftp(&mut self) -> Result<String, String> {
        if !self.sftp_sessions.is_empty() {
            return Err("SFTP session already open".to_string());
        }

        let mut handle = self.handle.lock().await;
        let session = open_sftp_session(&mut handle)
            .await
            .map_err(|error| error.to_string())?;
        let session_id = session.id().to_string();

        self.sftp_sessions
            .insert(session_id.clone(), Arc::new(session));
        Ok(session_id)
    }

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

struct OutputBatch {
    bytes: Vec<u8>,
    last_flush: Option<tokio::time::Instant>,
}

impl OutputBatch {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(OUTPUT_CHUNK_BYTES),
            last_flush: None,
        }
    }

    fn deadline(&self) -> tokio::time::Instant {
        self.last_flush.unwrap_or_else(tokio::time::Instant::now) + SSH_OUTPUT_FLUSH_INTERVAL
    }

    async fn push(&mut self, mut data: &[u8], output: &OutputSession) -> Result<(), String> {
        // Idle output is never held for a batching timer.
        if self.bytes.is_empty()
            && self
                .last_flush
                .is_none_or(|last| last.elapsed() >= SSH_OUTPUT_FLUSH_INTERVAL)
        {
            output.send(data).await?;
            self.last_flush = Some(tokio::time::Instant::now());
            return Ok(());
        }
        while !data.is_empty() {
            let take = data.len().min(OUTPUT_CHUNK_BYTES - self.bytes.len());
            self.bytes.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.bytes.len() == OUTPUT_CHUNK_BYTES {
                self.flush(output).await?;
            }
        }
        Ok(())
    }

    async fn flush(&mut self, output: &OutputSession) -> Result<(), String> {
        if !self.bytes.is_empty() {
            output.send(&self.bytes).await?;
            self.bytes.clear();
            self.last_flush = Some(tokio::time::Instant::now());
        }
        Ok(())
    }
}

async fn read_loop(
    mut channel_read: ChannelReadHalf,
    session_id: String,
    app: AppHandle,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    output: Arc<OutputSession>,
) {
    let mut batch = OutputBatch::new();
    let error = loop {
        tokio::select! {
            biased;
            _ = output.stopped() => break None,
            _ = tokio::time::sleep_until(batch.deadline()), if !batch.bytes.is_empty() => {
                if let Err(error) = batch.flush(&output).await { break Some(error); }
            }
            message = channel_read.wait() => {
                match message {
                    Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                        if let Err(error) = batch.push(&data, &output).await { break Some(error); }
                    }
                    Some(ChannelMsg::Eof | ChannelMsg::Close) | None => break None,
                    Some(ChannelMsg::Failure) => break Some("SSH channel request failed".to_string()),
                    Some(_) => {}
                }
            }
        }
    };
    let flush_error = batch.flush(&output).await.err();
    let error = error.or(flush_error);
    if let Err(close_error) = output.close(error.clone()) {
        warn!(session_id, %close_error, "Failed to close terminal output");
    }
    remove_session_and_stop_port_forwards(sessions, &session_id, &app, error, true).await;
}

async fn run_shell_write_loop(
    channel_write: ChannelWriteHalf<Msg>,
    mut write_rx: mpsc::Receiver<SshWriteRequest>,
    output: Arc<OutputSession>,
) {
    while let Some(request) = tokio::select! {
        _ = output.stopped() => None,
        request = write_rx.recv() => request,
    } {
        let result = tokio::select! {
            _ = output.stopped() => Err("SSH session closed before completing input".to_string()),
            result = async {
                match request.operation {
                    SshWriteOperation::Data(data) => channel_write.data(&data[..]).await,
                    SshWriteOperation::Resize { cols, rows } => channel_write.window_change(cols, rows, 0, 0).await,
                }
            } => result.map_err(|e| format!("SSH terminal write failed: {e}")),
        };
        if let Err(error) = &result {
            output.fail(error.clone());
        }
        let failed = result.is_err();
        let _ = request.completion.send(result);
        if failed {
            break;
        }
    }
}

fn map_ssh_write_queue_error(error: mpsc::error::TrySendError<SshWriteRequest>) -> String {
    match error {
        mpsc::error::TrySendError::Full(_) => "SSH terminal input queue is full".to_string(),
        mpsc::error::TrySendError::Closed(_) => "SSH write loop is no longer available".to_string(),
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
        disk_read_bytes_per_second: value("disk_read_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
        disk_write_bytes_per_second: value("disk_write_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
        network_rx_bytes_per_second: value("network_rx_bytes_per_second")
            .and_then(|value| value.parse::<f64>().ok()),
        network_tx_bytes_per_second: value("network_tx_bytes_per_second")
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use crate::trust::{HostTrustMismatch, HostTrustPrompt, TrustCheck};

    use super::{map_connect_error, parse_host_system_info, OutputBatch, SshConnectResponse};

    #[tokio::test]
    async fn idle_output_is_immediate_and_burst_flush_preserves_final_bytes() {
        use crate::runtime::terminal_output::{TerminalOutput, OUTPUT_CHUNK_BYTES};
        use std::future::Future;
        use std::task::Poll;
        use tauri::ipc::{Channel, InvokeResponseBody};

        let frames = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received = frames.clone();
        let transport = TerminalOutput::default();
        transport
            .subscribe(Channel::new(move |body| {
                let InvokeResponseBody::Raw(bytes) = body else {
                    panic!("expected raw bytes")
                };
                received.lock().expect("frames").push(bytes);
                Ok(())
            }))
            .expect("subscribe");
        let output = transport
            .open(uuid::Uuid::new_v4().to_string())
            .expect("open");
        let mut batch = OutputBatch::new();
        {
            let first = batch.push(b"prompt", &output);
            tokio::pin!(first);
            assert!(matches!(
                std::future::poll_fn(|cx| Poll::Ready(first.as_mut().poll(cx))).await,
                Poll::Ready(Ok(()))
            ));
        }
        assert_eq!(&frames.lock().expect("frames")[0][37..], b"prompt");
        batch
            .push(&vec![7; OUTPUT_CHUNK_BYTES + 3], &output)
            .await
            .expect("burst");
        batch.flush(&output).await.expect("flush EOF");
        output.close(None).expect("close");
        let frames = frames.lock().expect("frames");
        assert!(frames
            .iter()
            .all(|frame| frame.len() <= 37 + OUTPUT_CHUNK_BYTES));
        let bytes: Vec<u8> = frames
            .iter()
            .filter(|frame| frame[0] == 0)
            .flat_map(|frame| frame[37..].iter().copied())
            .collect();
        assert_eq!(&bytes[..6], b"prompt");
        assert_eq!(&bytes[6..], vec![7; OUTPUT_CHUNK_BYTES + 3]);
        assert_eq!(frames.last().expect("close")[0], 1);
    }

    #[test]
    fn parse_host_system_info_reads_io_metrics() {
        let info = parse_host_system_info(
            "hostname=buildbox\nos=Linux\ncpu_usage_percent=17.4\nmemory_total_bytes=17179869184\nmemory_used_bytes=8589934592\nmemory_usage_percent=50.0\ndisk_read_bytes_per_second=4096\ndisk_write_bytes_per_second=8192\nnetwork_rx_bytes_per_second=123456\nnetwork_tx_bytes_per_second=654321\n",
        );

        assert_eq!(info.hostname, Some("buildbox".to_string()));
        assert_eq!(info.os, Some("Linux".to_string()));
        assert_eq!(info.cpu_usage_percent, Some(17.4));
        assert_eq!(info.memory_total_bytes, Some(17_179_869_184.0));
        assert_eq!(info.memory_used_bytes, Some(8_589_934_592.0));
        assert_eq!(info.memory_usage_percent, Some(50.0));
        assert_eq!(info.disk_read_bytes_per_second, Some(4096.0));
        assert_eq!(info.disk_write_bytes_per_second, Some(8192.0));
        assert_eq!(info.network_rx_bytes_per_second, Some(123_456.0));
        assert_eq!(info.network_tx_bytes_per_second, Some(654_321.0));
    }

    #[test]
    fn parse_host_system_info_ignores_missing_or_invalid_io_metrics() {
        let info = parse_host_system_info(
            "hostname=\nos=Darwin\ncpu_usage_percent=bad\nmemory_total_bytes=\ndisk_read_bytes_per_second=not-a-number\ndisk_write_bytes_per_second=\nnetwork_rx_bytes_per_second=42\n",
        );

        assert_eq!(info.hostname, None);
        assert_eq!(info.os, Some("Darwin".to_string()));
        assert_eq!(info.cpu_usage_percent, None);
        assert_eq!(info.memory_total_bytes, None);
        assert_eq!(info.disk_read_bytes_per_second, None);
        assert_eq!(info.disk_write_bytes_per_second, None);
        assert_eq!(info.network_rx_bytes_per_second, Some(42.0));
        assert_eq!(info.network_tx_bytes_per_second, None);
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
