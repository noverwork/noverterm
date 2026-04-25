use tauri::{AppHandle, State};

use crate::runtime::local::LocalSessionManager;
use crate::runtime::ssh::{AuthMethod, SshConnectResponse, SshSessionManager};
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
        .connect(
            app,
            trust_store.inner().clone(),
            input.host,
            input.port,
            input.username,
            auth_method,
            cols,
            rows,
        )
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
    session_id: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.disconnect(&session_id).await
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
