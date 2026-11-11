//! Runs non-interactive commands and SFTP operations on a host for callers
//! such as the MCP server that need a clean result rather than a terminal
//! screen.

use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use russh::client::Handle;
use russh::{ChannelMsg, Disconnect};
use tokio::sync::{mpsc, Notify};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use uuid::Uuid;

use crate::runtime::sftp::{open_sftp_session, SftpSession, TransferCancellation};
use crate::trust::{SshTrustStore, TrustCheck};

use super::authentication::{authenticate_session, client_config, ClientHandler};
use super::jump::{open_transport, OpenTransportError, SshHopInput};
use super::AuthMethod;

/// Each stream keeps at most this much; the rest is dropped and flagged.
const OUTPUT_LIMIT: usize = 64 * 1024;
/// Unread output a background command keeps; older bytes are dropped.
const BACKGROUND_OUTPUT_LIMIT: usize = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ExecTarget {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    pub jumps: Vec<SshHopInput>,
}

#[derive(Debug, Default)]
pub struct ExecOutput {
    pub exit_status: Option<u32>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

// ponytail: one SSH connection per command; pool them if agents run many in a row.
pub async fn exec_command(
    trust_store: &SshTrustStore,
    target: ExecTarget,
    command: &str,
    run_timeout: Duration,
) -> Result<ExecOutput, String> {
    let session = connect(trust_store, target).await?;

    let mut channel = session
        .channel_open_session()
        .await
        .map_err(|error| format!("Failed to open a channel: {error}"))?;
    channel
        .exec(true, command)
        .await
        .map_err(|error| format!("Failed to start the command: {error}"))?;

    let mut output = ExecOutput::default();
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let collect = async {
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => append(&mut stdout, &data, &mut output.truncated),
                ChannelMsg::ExtendedData { data, .. } => {
                    append(&mut stderr, &data, &mut output.truncated)
                }
                ChannelMsg::ExitStatus { exit_status } => output.exit_status = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
    };
    let finished = timeout(run_timeout, collect).await.is_ok();
    let _ = session
        .disconnect(Disconnect::ByApplication, "Command finished", "")
        .await;
    if !finished {
        return Err(format!(
            "The command did not finish within {} seconds and was stopped",
            run_timeout.as_secs()
        ));
    }

    output.stdout = String::from_utf8_lossy(&stdout).into_owned();
    output.stderr = String::from_utf8_lossy(&stderr).into_owned();
    Ok(output)
}

async fn connect(
    trust_store: &SshTrustStore,
    target: ExecTarget,
) -> Result<Handle<ClientHandler>, String> {
    let exec_id = Uuid::new_v4().to_string();
    let config = client_config(None);
    let transport = timeout(
        CONNECT_TIMEOUT,
        open_transport(
            config,
            &target.host,
            target.port,
            target.jumps,
            trust_store,
            &exec_id,
        ),
    )
    .await
    .map_err(|_| format!("Timed out connecting to {}", target.host))?;

    let mut session = match transport {
        Ok(session) => session,
        Err(OpenTransportError::Connect { error, trust_check }) => {
            return Err(match trust_check.lock().await.clone() {
                Some(TrustCheck::TrustRequired(prompt)) => format!(
                    "The host key of {}:{} is not trusted yet. Connect to it once in noverterm to verify and trust it.",
                    prompt.host, prompt.port
                ),
                Some(TrustCheck::TrustMismatch(mismatch)) => format!(
                    "The host key of {}:{} changed since it was trusted. Refusing to connect; check it in noverterm.",
                    mismatch.host, mismatch.port
                ),
                _ => format!("Failed to connect: {error}"),
            });
        }
        Err(OpenTransportError::Other(error)) => return Err(error),
    };
    authenticate_session(&mut session, &target.username, target.auth, &exec_id).await?;
    Ok(session)
}

/// An SFTP session on its own SSH connection.
pub struct RemoteSftp {
    session: Handle<ClientHandler>,
    pub sftp: SftpSession,
}

pub async fn open_sftp(
    trust_store: &SshTrustStore,
    target: ExecTarget,
) -> Result<RemoteSftp, String> {
    let mut session = connect(trust_store, target).await?;
    match open_sftp_session(&mut session).await {
        Ok(sftp) => Ok(RemoteSftp { session, sftp }),
        Err(error) => {
            let _ = session
                .disconnect(Disconnect::ByApplication, "SFTP failed", "")
                .await;
            Err(error.to_string())
        }
    }
}

impl RemoteSftp {
    pub async fn close(self) {
        let _ = self.sftp.close().await;
        let _ = self
            .session
            .disconnect(Disconnect::ByApplication, "SFTP finished", "")
            .await;
    }
}

/// Copies one file or directory between this machine and the host over SFTP.
/// Returns the bytes moved.
pub async fn transfer(
    trust_store: &SshTrustStore,
    target: ExecTarget,
    upload: bool,
    local_path: &str,
    remote_path: &str,
    run_timeout: Duration,
) -> Result<u64, String> {
    let remote = open_sftp(trust_store, target).await?;
    let id = Uuid::new_v4().to_string();
    let cancel = TransferCancellation::new();
    let copy = async {
        if upload {
            remote
                .sftp
                .upload(local_path, remote_path, id, cancel, None)
                .await
        } else {
            remote
                .sftp
                .download(remote_path, local_path, id, cancel, None)
                .await
        }
    };
    let result = timeout(run_timeout, copy).await;
    remote.close().await;
    result
        .map_err(|_| {
            format!(
                "The transfer did not finish within {} seconds and was stopped",
                run_timeout.as_secs()
            )
        })?
        .map_err(|error| error.to_string())
}

/// A command left running on its own connection. Output piles up until read;
/// dropping it stops the command.
pub struct BackgroundCommand {
    input: mpsc::UnboundedSender<Option<Vec<u8>>>,
    state: Arc<StdMutex<BackgroundState>>,
    changed: Arc<Notify>,
    task: JoinHandle<()>,
}

#[derive(Debug, Default)]
pub struct BackgroundOutput {
    pub output: String,
    /// Older unread output was dropped to stay within the limit.
    pub dropped: bool,
    pub finished: bool,
    pub exit_status: Option<u32>,
}

#[derive(Default)]
struct BackgroundState {
    output: Vec<u8>,
    dropped: bool,
    finished: bool,
    exit_status: Option<u32>,
}

pub async fn spawn_command(
    trust_store: &SshTrustStore,
    target: ExecTarget,
    command: &str,
    tty: bool,
) -> Result<BackgroundCommand, String> {
    let session = connect(trust_store, target).await?;
    let channel = session
        .channel_open_session()
        .await
        .map_err(|error| format!("Failed to open a channel: {error}"))?;
    if tty {
        channel
            .request_pty(true, "xterm-256color", 120, 40, 0, 0, &[])
            .await
            .map_err(|error| format!("Failed to request a TTY: {error}"))?;
    }
    channel
        .exec(true, command)
        .await
        .map_err(|error| format!("Failed to start the command: {error}"))?;

    let (input, mut input_rx) = mpsc::unbounded_channel::<Option<Vec<u8>>>();
    let state = Arc::new(StdMutex::new(BackgroundState::default()));
    let changed = Arc::new(Notify::new());
    let (task_state, task_changed) = (state.clone(), changed.clone());
    let task = tokio::spawn(async move {
        let (mut read, write) = channel.split();
        loop {
            tokio::select! {
                message = read.wait() => match message {
                    Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                        let mut state = task_state.lock().unwrap();
                        state.output.extend_from_slice(&data);
                        let excess = state.output.len().saturating_sub(BACKGROUND_OUTPUT_LIMIT);
                        if excess > 0 {
                            state.output.drain(..excess);
                            state.dropped = true;
                        }
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        task_state.lock().unwrap().exit_status = Some(exit_status);
                    }
                    Some(ChannelMsg::Close) | None => break,
                    Some(_) => continue,
                },
                Some(data) = input_rx.recv() => {
                    let sent = match data {
                        Some(data) => write.data(&data[..]).await,
                        None => write.eof().await,
                    };
                    if sent.is_err() {
                        break;
                    }
                    continue;
                }
            }
            task_changed.notify_one();
        }
        task_state.lock().unwrap().finished = true;
        task_changed.notify_one();
        let _ = session
            .disconnect(Disconnect::ByApplication, "Command finished", "")
            .await;
    });
    Ok(BackgroundCommand {
        input,
        state,
        changed,
        task,
    })
}

impl BackgroundCommand {
    /// Sends `data` to the command's stdin; `None` closes stdin.
    pub fn write(&self, data: Option<Vec<u8>>) -> Result<(), String> {
        self.input
            .send(data)
            .map_err(|_| "The command already finished".to_string())
    }

    pub fn finished(&self) -> bool {
        self.state.lock().unwrap().finished
    }

    /// Takes the unread output, first waiting up to `wait` for some to arrive.
    pub async fn read(&self, wait: Duration) -> BackgroundOutput {
        let idle = {
            let state = self.state.lock().unwrap();
            state.output.is_empty() && !state.finished
        };
        if idle {
            let _ = timeout(wait, self.changed.notified()).await;
        }
        let mut state = self.state.lock().unwrap();
        BackgroundOutput {
            output: String::from_utf8_lossy(&std::mem::take(&mut state.output)).into_owned(),
            dropped: std::mem::take(&mut state.dropped),
            finished: state.finished,
            exit_status: state.exit_status,
        }
    }
}

impl Drop for BackgroundCommand {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn append(buffer: &mut Vec<u8>, data: &[u8], truncated: &mut bool) {
    let room = OUTPUT_LIMIT.saturating_sub(buffer.len());
    if data.len() > room {
        *truncated = true;
    }
    buffer.extend_from_slice(&data[..data.len().min(room)]);
}
