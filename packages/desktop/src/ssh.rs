use russh::client::{self, Handle, Msg};
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, Disconnect};
use russh::keys::ssh_key::PublicKey;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

pub(crate) struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> impl std::future::Future<Output = Result<bool, Self::Error>> + Send {
        async { Ok(true) }
    }
}

pub struct SshSession {
    pub handle: Handle<ClientHandler>,
    pub channel_write: ChannelWriteHalf<Msg>,
}

#[derive(Default)]
pub struct SshSessionManager {
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
}

impl SshSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(
        &self,
        app: AppHandle,
        host: String,
        port: u16,
        user: String,
        auth: AuthMethod,
        cols: u32,
        rows: u32,
    ) -> Result<String, String> {
        let session_id = Uuid::new_v4().to_string();
        info!(session_id, host, port, user, "Connecting to SSH server");

        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(0)),
            ..<_>::default()
        };
        let config = Arc::new(config);
        let handler = ClientHandler;

        let mut session = client::connect(config, (host.clone(), port), handler)
            .await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        match auth {
            AuthMethod::Password(password) => {
                let auth_res = session
                    .authenticate_password(user.clone(), password)
                    .await
                    .map_err(|e| format!("Password authentication failed: {}", e))?;
                if !auth_res.success() {
                    return Err("Password authentication rejected".to_string());
                }
            }
            AuthMethod::KeyFile(key_path) => {
                let key = load_key_pair(&key_path)?;
                let hash_alg = session
                    .best_supported_rsa_hash()
                    .await
                    .map_err(|e| format!("Failed to get supported RSA hash: {}", e))?
                    .flatten();
                let auth_res = session
                    .authenticate_publickey(
                        user.clone(),
                        russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
                    )
                    .await
                    .map_err(|e| format!("Key authentication failed: {}", e))?;
                if !auth_res.success() {
                    return Err("Key authentication rejected".to_string());
                }
            }
        }

        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| format!("Failed to open channel: {}", e))?;

        channel
            .request_pty(true, "xterm-256color", cols, rows, 0, 0, &[])
            .await
            .map_err(|e| format!("Failed to request PTY: {}", e))?;

        channel
            .exec(true, "exec $SHELL || exec bash")
            .await
            .map_err(|e| format!("Failed to exec shell: {}", e))?;

        let (channel_read, channel_write) = channel.split();

        let sid = session_id.clone();
        let sessions = self.sessions.clone();

        tokio::spawn(async move {
            read_loop(channel_read, sid, app, sessions).await;
        });

        self.sessions.lock().await.insert(
            session_id.clone(),
            SshSession {
                handle: session,
                channel_write,
            },
        );

        info!(session_id, "SSH session established");
        Ok(session_id)
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

        Ok(())
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.remove(session_id) {
            info!(session_id, "Disconnecting SSH session");
            let _ = session.channel_write.close().await;
            let _ = session
                .handle
                .disconnect(Disconnect::ByApplication, "Client disconnected", "")
                .await;
        }
        Ok(())
    }
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
                    sessions.lock().await.remove(&session_id);
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
                    sessions.lock().await.remove(&session_id);
                    break;
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    info!(session_id, "Exit status: {}", exit_status);
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
                sessions.lock().await.remove(&session_id);
                break;
            }
        }
    }
}

pub enum AuthMethod {
    Password(String),
    KeyFile(PathBuf),
}

fn load_key_pair(path: &PathBuf) -> Result<russh::keys::PrivateKey, String> {
    let key = russh::keys::load_secret_key(path, None)
        .map_err(|e| format!("Failed to load key file: {}", e))?;
    Ok(key)
}

#[derive(Clone, Serialize)]
pub struct SshOutputEvent {
    pub session_id: String,
    pub output: String,
    pub closed: bool,
}
