use russh::client::{self, Handle, Msg};
use russh::keys::ssh_key::HashAlg;
use russh::keys::ssh_key::PublicKey;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, Disconnect};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::trust::{HostTrustMismatch, HostTrustPrompt, SshTrustStore, TrustCheck};

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
    pub(crate) handle: Handle<ClientHandler>,
    pub(crate) channel_write: ChannelWriteHalf<Msg>,
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
        trust_store: SshTrustStore,
        host: String,
        port: u16,
        user: String,
        auth: AuthMethod,
        cols: u32,
        rows: u32,
    ) -> Result<SshConnectResponse, String> {
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

        match auth {
            AuthMethod::Password(password) => {
                info!(session_id, user, "Authenticating with password");
                let auth_res = session
                    .authenticate_password(user.clone(), password)
                    .await
                    .map_err(|e| format!("Password authentication failed: {}", e))?;
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
                info!(session_id, user, "Key authentication succeeded");
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
                if auth_res.success() {
                    info!(session_id, user, "Key authentication succeeded");
                } else {
                    info!(session_id, user, "Key auth partial, trying password");
                    let auth_res2 = session
                        .authenticate_password(user.clone(), password)
                        .await
                        .map_err(|e| format!("Password authentication failed: {}", e))?;
                    if !auth_res2.success() {
                        return Err("Key + password authentication rejected".to_string());
                    }
                    info!(session_id, user, "Key + password authentication succeeded");
                }
            }
        }

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
                handle: session,
                channel_write,
            },
        );

        info!(session_id, "SSH session established");
        Ok(SshConnectResponse::Connected { session_id })
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
                sessions.lock().await.remove(&session_id);
                break;
            }
        }
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

fn load_key_pair(
    private_key: &str,
    passphrase: Option<&str>,
) -> Result<russh::keys::PrivateKey, String> {
    let key = russh::keys::PrivateKey::from_openssh(private_key)
        .map_err(|error| format!("Failed to parse SSH key: {error}"))?;

    if key.is_encrypted() {
        let passphrase = passphrase
            .ok_or_else(|| "SSH key requires a passphrase, but none was provided".to_string())?;
        key.decrypt(passphrase)
            .map_err(|error| format!("Failed to decrypt SSH key: {error}"))
    } else {
        Ok(key)
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
