//! noverterm relay: gives noverterm Links a public URL at `<name>.<BASE_DOMAIN>`.
//!
//! Clients authenticate with a key listed in `AUTHORIZED_KEYS` and request
//! `ssh -R <name>:80:...`; visitors reach them through Caddy on the HTTP port.

mod ingress;
mod routes;
mod ssh;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

use routes::Routes;
use ssh::State;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let base_domain: Arc<str> = std::env::var("BASE_DOMAIN")
        .context("BASE_DOMAIN is required, e.g. link.example.com")?
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .into();
    let host_key = PathBuf::from(env_or("HOST_KEY_PATH", "/data/host_key"));
    let state = Arc::new(State {
        routes: Routes::new(),
        authorized_keys: env_or("AUTHORIZED_KEYS", "/data/authorized_keys").into(),
    });
    info!(%base_domain, "starting noverterm relay");

    tokio::select! {
        result = ssh::serve(port("SSH_PORT", 2222)?, &host_key, state.clone()) => result,
        result = ingress::serve_visitors(port("INGRESS_PORT", 8080)?, base_domain.clone(), state.clone()) => result,
        result = ingress::serve_tls_ask(port("ASK_PORT", 8081)?, base_domain, state) => result,
        _ = tokio::signal::ctrl_c() => Ok(()),
    }
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn port(name: &str, default: u16) -> Result<u16> {
    std::env::var(name).map_or(Ok(default), |value| {
        value
            .parse()
            .with_context(|| format!("{name} must be a port number"))
    })
}
