use russh::client::{self, AuthResult, Handle};
use russh::keys::ssh_key::HashAlg;
use russh::keys::ssh_key::PublicKey;
use russh::Disconnect;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;

use crate::trust::{SshTrustStore, TrustCheck};

use super::keys::{is_rsa_key, load_key_pair, rsa_hash_candidates};

struct ClientHandler {
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PortForwardCreateInput {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: PortForwardAuth,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PortForwardAuth {
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

#[derive(Debug, Clone, Serialize, Type)]
pub struct PortForwardStatus {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub state: PortForwardState,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PortForwardState {
    Connecting,
    Listening,
    Stopped,
    Error,
}

struct PortForwardEntry {
    status: PortForwardStatus,
    ssh_task: JoinHandle<()>,
}

#[derive(Default)]
pub struct PortForwardManager {
    forwards: Arc<Mutex<HashMap<String, PortForwardEntry>>>,
}

impl PortForwardManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(
        &self,
        app: AppHandle,
        input: PortForwardCreateInput,
        trust_store: SshTrustStore,
    ) -> Result<PortForwardStatus, String> {
        let input = normalize_input(input)?;
        let forward_id = Uuid::new_v4().to_string();

        let status = PortForwardStatus {
            id: forward_id.clone(),
            name: input.name.clone(),
            host: input.host.clone(),
            port: input.port,
            username: input.username.clone(),
            bind_host: input.bind_host.clone(),
            bind_port: input.bind_port,
            target_host: input.target_host.clone(),
            target_port: input.target_port,
            state: PortForwardState::Connecting,
            error: None,
        };

        let forwards = self.forwards.clone();
        let app_clone = app.clone();
        let status_clone = status.clone();
        let mut entries = self.forwards.lock().await;

        let ssh_task = tokio::spawn(async move {
            run_port_forward(app_clone, input, trust_store, status_clone, forwards).await;
        });

        let entry = PortForwardEntry {
            status: status.clone(),
            ssh_task,
        };

        let forward_id_clone = forward_id.clone();
        entries.insert(forward_id, entry);
        emit_status_event(&app, &status);
        drop(entries);

        info!(
            forward_id = forward_id_clone,
            "Started independent port forward"
        );
        Ok(status)
    }

    pub async fn stop(
        &self,
        app: AppHandle,
        forward_id: &str,
    ) -> Result<PortForwardStatus, String> {
        let entry = self
            .forwards
            .lock()
            .await
            .remove(forward_id)
            .ok_or_else(|| format!("Port forward not found: {forward_id}"))?;

        entry.ssh_task.abort();

        let mut status = entry.status;
        status.state = PortForwardState::Stopped;
        status.error = None;

        emit_status_event(&app, &status);
        info!(forward_id, "Stopped port forward");
        Ok(status)
    }

    pub async fn list(&self) -> Vec<PortForwardStatus> {
        self.forwards
            .lock()
            .await
            .values()
            .map(|entry| entry.status.clone())
            .collect()
    }
}

async fn update_stored_status(
    forwards: &Arc<Mutex<HashMap<String, PortForwardEntry>>>,
    status: &PortForwardStatus,
) -> bool {
    if let Some(entry) = forwards.lock().await.get_mut(&status.id) {
        entry.status = status.clone();
        return true;
    }

    false
}

async fn emit_and_store_status(
    app: &AppHandle,
    forwards: &Arc<Mutex<HashMap<String, PortForwardEntry>>>,
    status: &PortForwardStatus,
) {
    if update_stored_status(forwards, status).await {
        emit_status_event(app, status);
    }
}

async fn run_port_forward(
    app: AppHandle,
    input: PortForwardCreateInput,
    trust_store: SshTrustStore,
    mut status: PortForwardStatus,
    forwards: Arc<Mutex<HashMap<String, PortForwardEntry>>>,
) {
    let config = client::Config {
        inactivity_timeout: None,
        ..<_>::default()
    };
    let config = Arc::new(config);
    let trust_check = Arc::new(Mutex::new(None));
    let handler = ClientHandler::new(
        input.host.clone(),
        input.port,
        trust_store,
        trust_check.clone(),
    );

    info!(
        forward_id = %status.id,
        host = %input.host,
        port = input.port,
        "Connecting to SSH server"
    );

    let mut session = match client::connect(config, (input.host.clone(), input.port), handler).await
    {
        Ok(session) => session,
        Err(error) => {
            let error_msg = map_connect_error(&error, &trust_check).await;
            warn!(forward_id = %status.id, "SSH connect failed: {}", error_msg);
            status.state = PortForwardState::Error;
            status.error = Some(error_msg);
            emit_and_store_status(&app, &forwards, &status).await;
            forwards.lock().await.remove(&status.id);
            return;
        }
    };

    info!(forward_id = %status.id, "SSH transport established, authenticating");

    match authenticate(&mut session, &input).await {
        Ok(()) => {}
        Err(error) => {
            warn!(forward_id = %status.id, "Authentication failed: {}", error);
            status.state = PortForwardState::Error;
            status.error = Some(error);
            emit_and_store_status(&app, &forwards, &status).await;
            forwards.lock().await.remove(&status.id);
            return;
        }
    }

    info!(forward_id = %status.id, "Authentication succeeded");

    let bind_host = input.bind_host.clone();
    let bind_port = input.bind_port;
    let listener = match TcpListener::bind((bind_host.as_str(), bind_port)).await {
        Ok(listener) => listener,
        Err(error) => {
            let error_msg = format!(
                "Failed to bind on {}:{}: {}",
                input.bind_host, input.bind_port, error
            );
            warn!(forward_id = %status.id, "{}", error_msg);
            status.state = PortForwardState::Error;
            status.error = Some(error_msg);
            emit_and_store_status(&app, &forwards, &status).await;
            forwards.lock().await.remove(&status.id);
            return;
        }
    };

    let actual_bind_port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(error) => {
            let error_msg = format!("Failed to get local address: {}", error);
            warn!(forward_id = %status.id, "{}", error_msg);
            status.state = PortForwardState::Error;
            status.error = Some(error_msg);
            emit_and_store_status(&app, &forwards, &status).await;
            forwards.lock().await.remove(&status.id);
            return;
        }
    };

    status.bind_port = actual_bind_port;
    status.state = PortForwardState::Listening;
    status.error = None;
    emit_and_store_status(&app, &forwards, &status).await;

    info!(
        forward_id = %status.id,
        bind_host = %input.bind_host,
        bind_port = actual_bind_port,
        target_host = %input.target_host,
        target_port = input.target_port,
        "Port forward listening"
    );

    let handle = Arc::new(Mutex::new(session));
    let target_host = input.target_host.clone();
    let target_port = input.target_port;
    let forward_id = status.id.clone();
    let app_clone = app.clone();
    let status_for_error = status.clone();

    run_listener_loop(
        listener,
        handle.clone(),
        target_host,
        target_port,
        app_clone,
        status_for_error,
        forwards.clone(),
    )
    .await;

    let _ = handle
        .lock()
        .await
        .disconnect(Disconnect::ByApplication, "Port forward stopped", "")
        .await;

    forwards.lock().await.remove(&forward_id);
}

async fn run_listener_loop(
    listener: TcpListener,
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    target_host: String,
    target_port: u16,
    app: AppHandle,
    mut status: PortForwardStatus,
    forwards: Arc<Mutex<HashMap<String, PortForwardEntry>>>,
) {
    loop {
        match listener.accept().await {
            Ok((local_stream, originator_addr)) => {
                let handle_clone = handle.clone();
                let target_host_clone = target_host.clone();
                let target_port_clone = target_port;
                let app_clone = app.clone();
                let mut status_clone = status.clone();
                let forwards_clone = forwards.clone();

                tokio::spawn(async move {
                    match handle_forward_connection(
                        local_stream,
                        originator_addr,
                        handle_clone,
                        target_host_clone,
                        target_port_clone,
                        app_clone.clone(),
                        status_clone.clone(),
                        forwards_clone.clone(),
                    )
                    .await
                    {
                        Ok(()) => {}
                        Err(error) => {
                            warn!(
                                forward_id = %status_clone.id,
                                "Forward connection failed: {}",
                                error
                            );
                            status_clone.state = PortForwardState::Listening;
                            status_clone.error = Some(error);
                            emit_and_store_status(&app_clone, &forwards_clone, &status_clone).await;
                        }
                    }
                });
            }
            Err(error) => {
                let error_msg = format!("Listener accept failed: {}", error);
                warn!(forward_id = %status.id, "{}", error_msg);
                status.state = PortForwardState::Error;
                status.error = Some(error_msg);
                emit_and_store_status(&app, &forwards, &status).await;
                break;
            }
        }
    }
}

async fn handle_forward_connection(
    mut local_stream: tokio::net::TcpStream,
    originator_addr: SocketAddr,
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    target_host: String,
    target_port: u16,
    app: AppHandle,
    mut status: PortForwardStatus,
    forwards: Arc<Mutex<HashMap<String, PortForwardEntry>>>,
) -> Result<(), String> {
    let target = format!("{target_host}:{target_port}");
    let originator = originator_addr.to_string();
    let channel = handle
        .lock()
        .await
        .channel_open_direct_tcpip(
            target_host.clone(),
            u32::from(target_port),
            originator_addr.ip().to_string(),
            u32::from(originator_addr.port()),
        )
        .await
        .map_err(|e| {
            format!("Failed to open direct-tcpip channel to {target} from {originator}: {e}")
        })?;

    status.state = PortForwardState::Listening;
    status.error = None;
    emit_and_store_status(&app, &forwards, &status).await;

    let mut remote_stream = channel.into_stream();
    copy_bidirectional(&mut local_stream, &mut remote_stream)
        .await
        .map_err(|e| format!("Failed to pipe traffic: {e}"))?;

    Ok(())
}

async fn authenticate(
    session: &mut Handle<ClientHandler>,
    input: &PortForwardCreateInput,
) -> Result<(), String> {
    match &input.auth {
        PortForwardAuth::Password(password) => {
            let auth_res = session
                .authenticate_password(input.username.clone(), password)
                .await
                .map_err(|e| format!("Password authentication failed: {}", e))?;
            if !auth_res.success() {
                return Err("Password authentication rejected".to_string());
            }
        }
        PortForwardAuth::PublicKey {
            private_key,
            passphrase,
        } => {
            let key = load_key_pair(private_key, passphrase.as_deref())?;
            match authenticate_public_key(session, &input.username, key).await? {
                PublicKeyAuthOutcome::Success => {}
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
        PortForwardAuth::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        } => {
            let key = load_key_pair(private_key, passphrase.as_deref())?;
            match authenticate_public_key(session, &input.username, key).await? {
                PublicKeyAuthOutcome::Success => {
                    info!("Key authentication succeeded");
                }
                PublicKeyAuthOutcome::PartialSuccess | PublicKeyAuthOutcome::Rejected => {
                    info!("Trying password after key authentication");
                    let auth_res2 = session
                        .authenticate_password(input.username.clone(), password)
                        .await
                        .map_err(|e| format!("Password authentication failed: {}", e))?;
                    if !auth_res2.success() {
                        return Err("Key + password authentication rejected".to_string());
                    }
                }
            }
        }
    }
    Ok(())
}

enum PublicKeyAuthOutcome {
    Success,
    PartialSuccess,
    Rejected,
}

async fn authenticate_public_key(
    session: &mut Handle<ClientHandler>,
    username: &str,
    key: russh::keys::PrivateKey,
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
        info!(username, rsa_hash_alg = ?hash_alg, "Trying port-forward public key authentication");
        let auth_res = session
            .authenticate_publickey(
                username.to_string(),
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

fn normalize_input(input: PortForwardCreateInput) -> Result<PortForwardCreateInput, String> {
    let name = input.name.trim().to_string();
    let host = input.host.trim().to_string();
    let username = input.username.trim().to_string();
    let bind_host = input.bind_host.trim().to_string();
    let target_host = input.target_host.trim().to_string();

    if name.is_empty() {
        return Err("name is required".to_string());
    }
    if host.is_empty() {
        return Err("host is required".to_string());
    }
    if input.port == 0 {
        return Err("port must be greater than 0".to_string());
    }
    if username.is_empty() {
        return Err("username is required".to_string());
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

    Ok(PortForwardCreateInput {
        name,
        host,
        port: input.port,
        username,
        auth: input.auth,
        bind_host,
        bind_port: input.bind_port,
        target_host,
        target_port: input.target_port,
    })
}

fn is_loopback_bind_host(bind_host: &str) -> bool {
    matches!(bind_host, "127.0.0.1" | "localhost" | "::1")
}

async fn map_connect_error(
    error: &russh::Error,
    trust_check: &Arc<Mutex<Option<TrustCheck>>>,
) -> String {
    match trust_check.lock().await.clone() {
        Some(TrustCheck::TrustRequired(prompt)) => {
            format!("Trust required: {} ({})", prompt.host, prompt.fingerprint)
        }
        Some(TrustCheck::TrustMismatch(mismatch)) => {
            format!(
                "Trust mismatch: expected {} {}, got {} {}",
                mismatch.expected_algorithm,
                mismatch.expected_fingerprint,
                mismatch.presented_algorithm,
                mismatch.presented_fingerprint
            )
        }
        Some(TrustCheck::Trusted) | None => format!("Failed to connect: {error}"),
    }
}

fn emit_status_event(app: &AppHandle, status: &PortForwardStatus) {
    if let Err(e) = app.emit("port_forward_status", status.clone()) {
        warn!(
            forward_id = %status.id,
            "Failed to emit port_forward_status: {}",
            e
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{PortForwardManager, PortForwardState};

    #[test]
    fn port_forward_manager_is_constructible() {
        let _ = PortForwardManager::new();
    }

    #[test]
    fn port_forward_state_variants_serialize() {
        let connecting = PortForwardState::Connecting;
        let listening = PortForwardState::Listening;
        let stopped = PortForwardState::Stopped;
        let error = PortForwardState::Error;

        assert!(serde_json::to_string(&connecting).is_ok());
        assert!(serde_json::to_string(&listening).is_ok());
        assert!(serde_json::to_string(&stopped).is_ok());
        assert!(serde_json::to_string(&error).is_ok());
    }
}
