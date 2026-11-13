//! SSH side: authorized clients claim subdomains with `tcpip-forward`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{Algorithm, PrivateKey, PublicKey};
use russh::server::{self, Auth, Handle, Msg, Server as _, Session};
use russh::{Channel, MethodKind, MethodSet};
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::routes::{self, Routes};

/// Where a claimed subdomain's traffic goes: back down this SSH session, as a
/// `forwarded-tcpip` channel naming the address and port the client asked for.
#[derive(Clone)]
pub struct Target {
    pub handle: Handle,
    pub address: String,
    pub port: u32,
}

pub struct State {
    pub routes: Routes<Target>,
    pub authorized_keys: PathBuf,
}

struct Server {
    state: Arc<State>,
    next_id: Arc<AtomicUsize>,
    client_id: usize,
}

pub async fn serve(port: u16, host_key: &Path, state: Arc<State>) -> Result<()> {
    let mut methods = MethodSet::empty();
    methods.push(MethodKind::PublicKey);
    let config = Arc::new(server::Config {
        methods,
        keys: vec![load_or_create_host_key(host_key)?],
        auth_rejection_time: Duration::from_millis(500),
        auth_rejection_time_initial: Some(Duration::ZERO),
        ..Default::default()
    });
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!(port, "SSH listening");
    let mut root = Server {
        state,
        next_id: Arc::new(AtomicUsize::new(1)),
        client_id: 0,
    };
    root.run_on_socket(config, &listener)
        .await
        .context("SSH server failed")
}

fn load_or_create_host_key(path: &Path) -> Result<PrivateKey> {
    if path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(path)?.permissions().mode();
            if mode & 0o077 != 0 {
                bail!(
                    "host key {} must not be readable by others (mode {mode:o})",
                    path.display()
                );
            }
        }
        return PrivateKey::read_openssh_file(path)
            .with_context(|| format!("reading host key {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
    key.write_openssh_file(path, russh::keys::ssh_key::LineEnding::LF)
        .with_context(|| format!("writing host key {}", path.display()))?;
    info!(path = %path.display(), "created host key");
    Ok(key)
}

impl server::Server for Server {
    type Handler = Self;

    fn new_client(&mut self, peer: Option<std::net::SocketAddr>) -> Self {
        let client_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        info!(client_id, ?peer, "client connected");
        Self {
            state: self.state.clone(),
            next_id: self.next_id.clone(),
            client_id,
        }
    }

    fn handle_session_error(&mut self, error: russh::Error) {
        warn!(%error, "SSH session error");
    }
}

impl server::Handler for Server {
    type Error = russh::Error;

    async fn auth_publickey(&mut self, user: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        // Read on every attempt so edits apply without a restart.
        let listed = tokio::fs::read_to_string(&self.state.authorized_keys)
            .await
            .unwrap_or_else(|error| {
                warn!(%error, path = %self.state.authorized_keys.display(), "cannot read authorized keys");
                String::new()
            });
        if routes::is_authorized(&listed, key) {
            info!(client_id = self.client_id, user, "key accepted");
            Ok(Auth::Accept)
        } else {
            warn!(client_id = self.client_id, user, "key rejected");
            Ok(Auth::reject())
        }
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        // Tunnels only: no shell, no exec. Use `ssh -N`.
        Ok(false)
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let Some(name) = routes::subdomain(address) else {
            warn!(client_id = self.client_id, address, "invalid subdomain");
            return Ok(false);
        };
        if *port == 0 {
            *port = 80;
        }
        let target = Target {
            handle: session.handle(),
            address: address.to_string(),
            port: *port,
        };
        let claimed = self.state.routes.claim(&name, self.client_id, target);
        if claimed {
            info!(client_id = self.client_id, name, "tunnel up");
        } else {
            warn!(client_id = self.client_id, name, "subdomain taken");
        }
        Ok(claimed)
    }

    async fn cancel_tcpip_forward(
        &mut self,
        address: &str,
        _port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        if let Some(name) = routes::subdomain(address) {
            self.state.routes.release(&name, self.client_id);
        }
        Ok(true)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.state.routes.release_client(self.client_id);
    }
}
