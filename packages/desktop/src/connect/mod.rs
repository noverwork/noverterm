use crate::app::{AppHandle, State};

use crate::runtime::port_forward::{
    PortForwardAuth, PortForwardCreateInput as InternalPortForwardInput, PortForwardManager,
    PortForwardStatus,
};
use crate::runtime::ssh::{
    auth_method, SshConnectRequest, SshConnectResponse, SshHopInput, SshSessionManager,
};
use crate::trust::{HostTrustConfirmation, SshTrustStore};

#[derive(Debug, serde::Deserialize)]
pub struct DirectSshConnectInput {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    /// Jump hosts to tunnel through, outermost first.
    #[serde(default)]
    pub jumps: Vec<SshHopInput>,
}

pub async fn ssh_connect_direct(
    app: AppHandle,
    input: DirectSshConnectInput,
    ssh_manager: State<'_, SshSessionManager>,
    trust_store: State<'_, SshTrustStore>,
    cols: u32,
    rows: u32,
) -> Result<SshConnectResponse, String> {
    let auth_method = auth_method(input.password, input.private_key, input.passphrase)?;

    ssh_manager
        .connect(SshConnectRequest {
            app,
            trust_store: trust_store.inner().clone(),
            host: input.host,
            port: input.port,
            user: input.username,
            auth: auth_method,
            jumps: input.jumps,
            cols,
            rows,
        })
        .await
}

pub async fn ssh_confirm_host_trust(
    confirmation: HostTrustConfirmation,
    trust_store: State<'_, SshTrustStore>,
) -> Result<(), String> {
    trust_store.confirm(confirmation).await
}

#[derive(Debug, serde::Deserialize)]
pub struct PortForwardStartInput {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    /// Jump hosts to tunnel through, outermost first.
    #[serde(default)]
    pub jumps: Vec<SshHopInput>,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
}

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
                jumps: input.jumps,
                bind_host: input.bind_host,
                bind_port: input.bind_port,
                target_host: input.target_host,
                target_port: input.target_port,
            },
            trust_store.inner().clone(),
        )
        .await
}

pub async fn port_forward_stop(
    app: AppHandle,
    forward_id: String,
    pf_manager: State<'_, PortForwardManager>,
) -> Result<PortForwardStatus, String> {
    pf_manager.stop(app, &forward_id).await
}

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
