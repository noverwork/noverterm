//! Links: a local HTTP service published at `https://<name>.<relay>` through
//! the user's own relay (`packages/relay`), over an SSH remote forward.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle, Msg, Session};
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::ssh_key::{HashAlg, LineEnding};
use russh::keys::{Algorithm, PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use russh::{Channel, Disconnect};
use serde::{Deserialize, Serialize};
use shared::Setting;
use tokio::io::{copy_bidirectional, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::app::AppHandle;
use crate::store::settings::{get_setting, set_setting};
use crate::trust::{HostTrustConfirmation, SshTrustStore, TrustCheck};

/// Settings key of the relay, `domain[:port]`.
pub const RELAY_SETTING: &str = "link_relay";
const KEY_SETTING: &str = "link_private_key";
const RELAY_USER: &str = "link";
const DEFAULT_RELAY_PORT: u16 = 2222;
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStatus {
    pub id: String,
    pub url: String,
    pub state: LinkState,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkState {
    Connecting,
    Online,
    Error,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct LinkStartInput {
    pub id: String,
    pub subdomain: String,
    pub target_host: String,
    pub target_port: u16,
    /// `domain[:port]`; the port defaults to 2222.
    pub relay: String,
}

type Running = Arc<Mutex<HashMap<String, (LinkStatus, JoinHandle<()>)>>>;

#[derive(Default)]
pub struct LinkManager {
    running: Running,
}

impl LinkManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(
        &self,
        app: AppHandle,
        input: LinkStartInput,
        trust_store: SshTrustStore,
    ) -> Result<LinkStatus, String> {
        let (relay_host, relay_port) = parse_relay(&input.relay)?;
        let key = link_key(&app).await?;
        let status = LinkStatus {
            id: input.id.clone(),
            url: format!("https://{}.{relay_host}", input.subdomain),
            state: LinkState::Connecting,
            error: None,
        };
        let mut running = self.running.lock().await;
        if let Some((_, task)) = running.remove(&input.id) {
            task.abort();
        }
        let job = Job {
            app: app.clone(),
            input,
            relay_host,
            relay_port,
            key: Arc::new(key),
            trust_store,
            running: self.running.clone(),
            status: status.clone(),
        };
        running.insert(status.id.clone(), (status.clone(), tokio::spawn(job.run())));
        drop(running);
        emit(&app, &status);
        Ok(status)
    }

    pub async fn stop(&self, app: AppHandle, id: &str) -> Result<(), String> {
        let (mut status, task) = self
            .running
            .lock()
            .await
            .remove(id)
            .ok_or_else(|| format!("Link not running: {id}"))?;
        task.abort();
        status.state = LinkState::Stopped;
        status.error = None;
        emit(&app, &status);
        Ok(())
    }

    pub async fn list(&self) -> Vec<LinkStatus> {
        let running = self.running.lock().await;
        running.values().map(|(status, _)| status.clone()).collect()
    }
}

/// The public key the relay owner adds to `authorized_keys`.
pub async fn link_public_key(app: &AppHandle) -> Result<String, String> {
    let key = link_key(app).await?;
    let public = key.public_key().to_openssh().map_err(|e| e.to_string())?;
    Ok(format!("{public} noverterm-link"))
}

/// This install's Link key, created on first use and kept in the database.
async fn link_key(app: &AppHandle) -> Result<PrivateKey, String> {
    if let Some(setting) = get_setting(KEY_SETTING.to_string(), app.state()).await? {
        return PrivateKey::from_openssh(&setting.value).map_err(|e| format!("Link key: {e}"));
    }
    let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519).map_err(|e| e.to_string())?;
    let value = key
        .to_openssh(LineEnding::LF)
        .map_err(|e| e.to_string())?
        .to_string();
    set_setting(
        Setting {
            key: KEY_SETTING.to_string(),
            value,
        },
        app.state(),
    )
    .await?;
    Ok(key)
}

pub(crate) fn parse_relay(relay: &str) -> Result<(String, u16), String> {
    let relay = relay.trim();
    let (host, port) = match relay.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse()
                .map_err(|_| format!("Invalid relay port: {port}"))?,
        ),
        None => (relay, DEFAULT_RELAY_PORT),
    };
    if host.is_empty() {
        return Err("Set the relay domain first".to_string());
    }
    Ok((host.to_ascii_lowercase(), port))
}

struct Job {
    app: AppHandle,
    input: LinkStartInput,
    relay_host: String,
    relay_port: u16,
    key: Arc<PrivateKey>,
    trust_store: SshTrustStore,
    running: Running,
    status: LinkStatus,
}

/// Why a connection attempt ended.
enum Failure {
    /// Retrying can't help: the user has to change something.
    Fatal(String),
    Retry(String),
}

impl Job {
    async fn run(mut self) {
        let mut was_online = false;
        loop {
            let error = match self.connect().await {
                Ok(handle) => {
                    was_online = true;
                    self.set(LinkState::Online, None).await;
                    info!(link = %self.status.url, "Link online");
                    // ponytail: polls for a dropped session; keepalives close it within ~45s.
                    while !handle.is_closed() {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    "Connection to relay lost".to_string()
                }
                // A name refused right after a drop may still be held by our own
                // stale session on the relay; it frees up once that times out.
                Err(Failure::Fatal(error)) if !was_online => {
                    warn!(link = %self.status.url, %error, "Link failed");
                    self.set(LinkState::Error, Some(error)).await;
                    self.running.lock().await.remove(&self.status.id);
                    return;
                }
                Err(Failure::Fatal(error) | Failure::Retry(error)) => error,
            };
            self.set(LinkState::Connecting, Some(error)).await;
            tokio::time::sleep(RETRY_DELAY).await;
        }
    }

    async fn set(&mut self, state: LinkState, error: Option<String>) {
        self.status.state = state;
        self.status.error = error;
        if let Some((status, _)) = self.running.lock().await.get_mut(&self.status.id) {
            *status = self.status.clone();
        }
        emit(&self.app, &self.status);
    }

    async fn connect(&self) -> Result<Handle<LinkHandler>, Failure> {
        let config = Arc::new(client::Config {
            inactivity_timeout: None,
            keepalive_interval: Some(Duration::from_secs(15)),
            ..<_>::default()
        });
        let mismatch = Arc::new(Mutex::new(None));
        let handler = LinkHandler {
            relay_host: self.relay_host.clone(),
            relay_port: self.relay_port,
            trust_store: self.trust_store.clone(),
            mismatch: mismatch.clone(),
            target_host: self.input.target_host.clone(),
            target_port: self.input.target_port,
        };
        let address = (self.relay_host.as_str(), self.relay_port);
        let mut handle = match client::connect(config, address, handler).await {
            Ok(handle) => handle,
            Err(error) => {
                return Err(match mismatch.lock().await.take() {
                    Some(mismatch) => Failure::Fatal(mismatch),
                    None => Failure::Retry(format!("Cannot reach relay: {error}")),
                })
            }
        };

        let key = PrivateKeyWithHashAlg::new(self.key.clone(), None);
        let auth = handle
            .authenticate_publickey(RELAY_USER, key)
            .await
            .map_err(|error| Failure::Retry(format!("Authentication failed: {error}")))?;
        // Retried: adding the key to the relay takes effect without a restart,
        // so the Link comes up on its own once the owner has done it.
        if !auth.success() {
            return Err(Failure::Retry(
                "Relay rejected the Link key: add it to the relay's authorized_keys".to_string(),
            ));
        }

        if let Err(error) = handle.tcpip_forward(self.input.subdomain.clone(), 80).await {
            let _ = handle.disconnect(Disconnect::ByApplication, "", "").await;
            return Err(match error {
                russh::Error::RequestDenied => Failure::Fatal(format!(
                    "The name \"{}\" is already in use on this relay",
                    self.input.subdomain
                )),
                other => Failure::Retry(format!("Relay refused the tunnel: {other}")),
            });
        }
        Ok(handle)
    }
}

struct LinkHandler {
    relay_host: String,
    relay_port: u16,
    trust_store: SshTrustStore,
    mismatch: Arc<Mutex<Option<String>>>,
    target_host: String,
    target_port: u16,
}

impl client::Handler for LinkHandler {
    type Error = russh::Error;

    /// Trusts the relay's host key the first time, then holds it to that key.
    async fn check_server_key(&mut self, key: &PublicKey) -> Result<bool, Self::Error> {
        let algorithm = key.algorithm().to_string();
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        let check = self
            .trust_store
            .evaluate(&self.relay_host, self.relay_port, &algorithm, &fingerprint)
            .await;
        match check {
            TrustCheck::Trusted => Ok(true),
            TrustCheck::TrustRequired(_) => {
                let confirmation = HostTrustConfirmation {
                    host: self.relay_host.clone(),
                    port: self.relay_port,
                    algorithm,
                    fingerprint,
                };
                Ok(self.trust_store.confirm(confirmation).await.is_ok())
            }
            TrustCheck::TrustMismatch(mismatch) => {
                *self.mismatch.lock().await = Some(format!(
                    "Relay host key changed (expected {}, got {}). If you rebuilt the relay, \
                     remove it under Known Hosts.",
                    mismatch.expected_fingerprint, mismatch.presented_fingerprint
                ));
                Ok(false)
            }
        }
    }

    /// A visitor arrived: dial the local service and pipe the two together.
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let target = (self.target_host.clone(), self.target_port);
        tokio::spawn(async move {
            let mut visitor = channel.into_stream();
            match TcpStream::connect((target.0.as_str(), target.1)).await {
                Ok(mut local) => {
                    let _ = copy_bidirectional(&mut visitor, &mut local).await;
                }
                Err(error) => {
                    let body = format!(
                        "noverterm could not reach {}:{}: {error}",
                        target.0, target.1
                    );
                    let response = format!(
                        "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain; charset=utf-8\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = visitor.write_all(response.as_bytes()).await;
                    let _ = visitor.shutdown().await;
                }
            }
        });
        Ok(())
    }
}

fn emit(app: &AppHandle, status: &LinkStatus) {
    if let Err(error) = app.emit("link_status", status.clone()) {
        warn!(link = %status.id, "Failed to emit link_status: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::parse_relay;

    #[test]
    fn relay_port_defaults_to_2222() {
        assert_eq!(
            parse_relay("Link.Example.com").unwrap(),
            ("link.example.com".into(), 2222)
        );
        assert_eq!(
            parse_relay(" link.example.com:22 ").unwrap(),
            ("link.example.com".into(), 22)
        );
        assert!(parse_relay("").is_err());
        assert!(parse_relay("link.example.com:x").is_err());
    }
}
