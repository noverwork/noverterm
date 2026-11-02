use std::sync::Arc;

use russh::client::{self, Handle};
use tokio::sync::Mutex;
use tracing::info;

use crate::trust::{SshTrustStore, TrustCheck};

use super::authentication::{authenticate_session, ClientHandler};
use super::AuthMethod;

/// One bastion on the way to the target, in connect order (outermost first).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SshHopInput {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
}

pub(crate) enum OpenTransportError {
    /// Transport or host-key failure; `trust_check` holds the failing hop's verdict.
    Connect {
        error: russh::Error,
        trust_check: Arc<Mutex<Option<TrustCheck>>>,
    },
    Other(String),
}

/// Opens an unauthenticated SSH transport to `host:port`, tunneling through
/// each jump host in order (like `ssh -J`). Jump hosts are authenticated here;
/// the caller authenticates the returned target session.
///
/// Each tunnel channel holds a sender to its jump session, so the jump
/// sessions live exactly as long as the transport stacked on them.
// ponytail: hops open sequentially; no per-hop timeout beyond the russh config.
pub(crate) async fn open_transport(
    config: Arc<client::Config>,
    host: &str,
    port: u16,
    jumps: Vec<SshHopInput>,
    trust_store: &SshTrustStore,
    session_id: &str,
) -> Result<Handle<ClientHandler>, OpenTransportError> {
    let mut tunnel = None;
    for hop in jumps {
        let mut session = connect_hop(
            &config,
            tunnel.take(),
            &hop.host,
            hop.port,
            trust_store,
            session_id,
        )
        .await?;
        let auth = auth_method(hop.password, hop.private_key, hop.passphrase).map_err(|error| {
            OpenTransportError::Other(format!("Jump host {}: {error}", hop.host))
        })?;
        authenticate_session(&mut session, &hop.username, auth, session_id)
            .await
            .map_err(|error| {
                OpenTransportError::Other(format!("Jump host {}: {error}", hop.host))
            })?;
        tunnel = Some(session);
    }
    connect_hop(&config, tunnel, host, port, trust_store, session_id).await
}

async fn connect_hop(
    config: &Arc<client::Config>,
    tunnel: Option<Handle<ClientHandler>>,
    host: &str,
    port: u16,
    trust_store: &SshTrustStore,
    session_id: &str,
) -> Result<Handle<ClientHandler>, OpenTransportError> {
    let trust_check = Arc::new(Mutex::new(None));
    let handler = ClientHandler::new(
        host.to_string(),
        port,
        trust_store.clone(),
        trust_check.clone(),
    );
    let connected = match tunnel {
        None => client::connect(config.clone(), (host.to_string(), port), handler).await,
        Some(jump) => {
            info!(session_id, host, port, "Opening tunnel through jump host");
            let channel = jump
                .channel_open_direct_tcpip(host, u32::from(port), "127.0.0.1", 0)
                .await
                .map_err(|error| {
                    OpenTransportError::Other(format!(
                        "Jump host could not reach {host}:{port}: {error}"
                    ))
                })?;
            client::connect_stream(config.clone(), channel.into_stream(), handler).await
        }
    };
    connected.map_err(|error| OpenTransportError::Connect { error, trust_check })
}

pub(crate) fn auth_method(
    password: Option<String>,
    private_key: Option<String>,
    passphrase: Option<String>,
) -> Result<AuthMethod, String> {
    let non_empty = |value: Option<String>| {
        value
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    match (
        non_empty(password),
        non_empty(private_key),
        non_empty(passphrase),
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
