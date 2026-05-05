use tauri::{AppHandle, State};

use crate::runtime::local::LocalSessionManager;
use crate::runtime::port_forward::{
    PortForwardAuth, PortForwardCreateInput as InternalPortForwardInput, PortForwardManager,
    PortForwardStatus,
};
use crate::runtime::ssh::{
    AuthMethod, SshConnectRequest, SshConnectResponse, SshLocalPortForwardInput,
    SshPortForwardStatus, SshProbeHostInfoResponse, SshSessionManager,
};
use crate::trust::{HostTrustConfirmation, SshTrustStore};

#[derive(Debug, serde::Deserialize, specta::Type)]
pub struct DirectSshConnectInput {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_connect_direct(
    app: AppHandle,
    input: DirectSshConnectInput,
    ssh_manager: State<'_, SshSessionManager>,
    trust_store: State<'_, SshTrustStore>,
    cols: u32,
    rows: u32,
) -> Result<SshConnectResponse, String> {
    let auth_method = direct_auth_method(input.password, input.private_key, input.passphrase)?;

    ssh_manager
        .connect(SshConnectRequest {
            app,
            trust_store: trust_store.inner().clone(),
            host: input.host,
            port: input.port,
            user: input.username,
            auth: auth_method,
            cols,
            rows,
        })
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_confirm_host_trust(
    confirmation: HostTrustConfirmation,
    trust_store: State<'_, SshTrustStore>,
) -> Result<(), String> {
    trust_store.confirm(confirmation).await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_probe_host_info(
    input: DirectSshConnectInput,
    ssh_manager: State<'_, SshSessionManager>,
    trust_store: State<'_, SshTrustStore>,
) -> Result<SshProbeHostInfoResponse, String> {
    let auth_method = direct_auth_method(input.password, input.private_key, input.passphrase)?;

    ssh_manager
        .probe_host_info(
            trust_store.inner().clone(),
            input.host,
            input.port,
            input.username,
            auth_method,
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_write(
    session_id: String,
    data: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.write(&session_id, data.into_bytes()).await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_resize(
    session_id: String,
    cols: u32,
    rows: u32,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.resize(&session_id, cols, rows).await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_disconnect(
    app: AppHandle,
    session_id: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.disconnect(app, &session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_start_local_port_forward(
    app: AppHandle,
    input: SshLocalPortForwardInput,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<SshPortForwardStatus, String> {
    ssh_manager.start_local_port_forward(app, input).await
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_stop_port_forward(
    app: AppHandle,
    session_id: String,
    forward_id: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<SshPortForwardStatus, String> {
    ssh_manager
        .stop_port_forward(app, &session_id, &forward_id)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn local_connect(
    app: AppHandle,
    cols: u32,
    rows: u32,
    local_manager: State<'_, LocalSessionManager>,
) -> Result<String, String> {
    local_manager.connect(app, cols, rows).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_write(
    session_id: String,
    data: String,
    local_manager: State<'_, LocalSessionManager>,
) -> Result<(), String> {
    local_manager.write(&session_id, data.into_bytes()).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_resize(
    session_id: String,
    cols: u32,
    rows: u32,
    local_manager: State<'_, LocalSessionManager>,
) -> Result<(), String> {
    local_manager.resize(&session_id, cols, rows).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_disconnect(
    session_id: String,
    local_manager: State<'_, LocalSessionManager>,
) -> Result<(), String> {
    local_manager.disconnect(&session_id).await
}

#[derive(Debug, serde::Deserialize, specta::Type)]
pub struct PortForwardStartInput {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
}

#[tauri::command]
#[specta::specta]
pub async fn port_forward_start(
    app: AppHandle,
    input: PortForwardStartInput,
    pf_manager: State<'_, PortForwardManager>,
    trust_store: State<'_, SshTrustStore>,
) -> Result<PortForwardStatus, String> {
    let auth = port_forward_auth_method(input.password, input.private_key, input.passphrase)?;

    pf_manager
        .start(
            app,
            InternalPortForwardInput {
                name: input.name,
                host: input.host,
                port: input.port,
                username: input.username,
                auth,
                bind_host: input.bind_host,
                bind_port: input.bind_port,
                target_host: input.target_host,
                target_port: input.target_port,
            },
            trust_store.inner().clone(),
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn port_forward_stop(
    app: AppHandle,
    forward_id: String,
    pf_manager: State<'_, PortForwardManager>,
) -> Result<PortForwardStatus, String> {
    pf_manager.stop(app, &forward_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn port_forward_list(
    pf_manager: State<'_, PortForwardManager>,
) -> Result<Vec<PortForwardStatus>, String> {
    Ok(pf_manager.list().await)
}

fn port_forward_auth_method(
    password: Option<String>,
    private_key: Option<String>,
    passphrase: Option<String>,
) -> Result<PortForwardAuth, String> {
    match (
        password.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
        private_key.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
        passphrase.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
    ) {
        (Some(password), Some(private_key), passphrase) => {
            Ok(PortForwardAuth::PublicKeyAndPassword {
                private_key,
                passphrase,
                password,
            })
        }
        (None, Some(private_key), passphrase) => Ok(PortForwardAuth::PublicKey {
            private_key,
            passphrase,
        }),
        (Some(password), None, _) => Ok(PortForwardAuth::Password(password)),
        (None, None, _) => Err("password or private key is required".to_string()),
    }
}

fn direct_auth_method(
    password: Option<String>,
    private_key: Option<String>,
    passphrase: Option<String>,
) -> Result<AuthMethod, String> {
    match (
        password.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
        private_key.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
        passphrase.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }),
    ) {
        (Some(password), Some(private_key), passphrase) => Ok(AuthMethod::PublicKeyAndPassword {
            private_key,
            passphrase,
            password,
        }),
        (None, Some(private_key), passphrase) => Ok(AuthMethod::PublicKey {
            private_key,
            passphrase,
        }),
        (Some(password), None, _) => Ok(AuthMethod::Password(password)),
        (None, None, _) => Err("password or private key is required".to_string()),
    }
}
