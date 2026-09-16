use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, AuthResult, Handle};
use russh::keys::ssh_key::{HashAlg, PublicKey};
use tokio::sync::Mutex;
use tracing::info;

use crate::runtime::keys::{is_rsa_key, load_key_pair, rsa_hash_candidates};
use crate::trust::{SshTrustStore, TrustCheck};

use super::{AuthMethod, SshConnectResponse};

pub(super) fn client_config(inactivity_timeout: Option<Duration>) -> Arc<client::Config> {
    // No zlib: russh 0.59's zlib path stalls under high-throughput incompressible
    // data (a bulk SFTP upload of an already-compressed file), which wedges the
    // whole transport and drops the session mid-transfer. Terminal output is
    // low-volume, so losing compression there is negligible; OpenSSH's own client
    // defaults compression off for the same reasons.
    // ponytail: drop-zlib workaround; revisit if russh fixes the compressor.
    Arc::new(client::Config {
        inactivity_timeout,
        ..<_>::default()
    })
}

pub(crate) struct ClientHandler {
    host: String,
    port: u16,
    trust_store: SshTrustStore,
    trust_check: Arc<Mutex<Option<TrustCheck>>>,
}

impl ClientHandler {
    pub(super) fn new(
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

enum PublicKeyAuthOutcome {
    Success,
    PartialSuccess,
    Rejected,
}

pub(super) async fn authenticate_session(
    session: &mut Handle<ClientHandler>,
    user: &str,
    auth: AuthMethod,
    session_id: &str,
) -> Result<(), String> {
    match auth {
        AuthMethod::Password(password) => {
            info!(session_id, user, "Authenticating with password");
            let auth_res = session
                .authenticate_password(user.to_string(), password)
                .await
                .map_err(|error| format!("Password authentication failed: {error}"))?;
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
            match authenticate_public_key(session, user, key, session_id).await? {
                PublicKeyAuthOutcome::Success => {
                    info!(session_id, user, "Key authentication succeeded");
                }
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
            match authenticate_public_key(session, user, key, session_id).await? {
                PublicKeyAuthOutcome::Success => {
                    info!(session_id, user, "Key authentication succeeded");
                }
                PublicKeyAuthOutcome::PartialSuccess | PublicKeyAuthOutcome::Rejected => {
                    info!(session_id, user, "Trying password after key authentication");
                    let auth_res = session
                        .authenticate_password(user.to_string(), password)
                        .await
                        .map_err(|error| format!("Password authentication failed: {error}"))?;
                    if !auth_res.success() {
                        return Err("Key + password authentication rejected".to_string());
                    }
                    info!(session_id, user, "Key + password authentication succeeded");
                }
            }
        }
    }

    Ok(())
}

async fn authenticate_public_key(
    session: &mut Handle<ClientHandler>,
    user: &str,
    key: russh::keys::PrivateKey,
    session_id: &str,
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
        info!(
            session_id,
            user,
            rsa_hash_alg = ?hash_alg,
            "Trying public key authentication"
        );
        let auth_res = session
            .authenticate_publickey(
                user.to_string(),
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

pub(super) async fn map_connect_error(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::{HostTrustMismatch, HostTrustPrompt};

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
