use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct SshTrustStore {
    path: PathBuf,
    records: Arc<RwLock<HashMap<String, TrustedSshHost>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct HostTrustPrompt {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct HostTrustMismatch {
    pub host: String,
    pub port: u16,
    pub expected_algorithm: String,
    pub expected_fingerprint: String,
    pub presented_algorithm: String,
    pub presented_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub struct HostTrustConfirmation {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrustCheck {
    Trusted,
    TrustRequired(HostTrustPrompt),
    TrustMismatch(HostTrustMismatch),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TrustedSshHost {
    host: String,
    port: u16,
    algorithm: String,
    fingerprint: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TrustedSshHostsFile {
    records: Vec<TrustedSshHost>,
}

impl SshTrustStore {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let records = load_records(&path)?;

        Ok(Self {
            path,
            records: Arc::new(RwLock::new(records)),
        })
    }

    pub async fn confirm(&self, confirmation: HostTrustConfirmation) -> Result<(), String> {
        let record = TrustedSshHost {
            host: confirmation.host,
            port: confirmation.port,
            algorithm: confirmation.algorithm,
            fingerprint: confirmation.fingerprint,
        };

        let snapshot = {
            let mut records = self.records.write().await;
            records.insert(record_key(&record.host, record.port), record);
            snapshot_records(&records)
        };

        persist_records(&self.path, snapshot).await
    }

    pub(crate) async fn evaluate(
        &self,
        host: &str,
        port: u16,
        algorithm: &str,
        fingerprint: &str,
    ) -> TrustCheck {
        let records = self.records.read().await;
        let Some(record) = records.get(&record_key(host, port)) else {
            return TrustCheck::TrustRequired(HostTrustPrompt {
                host: host.to_string(),
                port,
                algorithm: algorithm.to_string(),
                fingerprint: fingerprint.to_string(),
            });
        };

        if record.algorithm == algorithm && record.fingerprint == fingerprint {
            TrustCheck::Trusted
        } else {
            TrustCheck::TrustMismatch(HostTrustMismatch {
                host: host.to_string(),
                port,
                expected_algorithm: record.algorithm.clone(),
                expected_fingerprint: record.fingerprint.clone(),
                presented_algorithm: algorithm.to_string(),
                presented_fingerprint: fingerprint.to_string(),
            })
        }
    }
}

fn load_records(path: &Path) -> Result<HashMap<String, TrustedSshHost>, String> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let contents = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let file = serde_json::from_str::<TrustedSshHostsFile>(&contents)
        .map_err(|error| error.to_string())?;

    Ok(file
        .records
        .into_iter()
        .map(|record| (record_key(&record.host, record.port), record))
        .collect())
}

fn snapshot_records(records: &HashMap<String, TrustedSshHost>) -> Vec<TrustedSshHost> {
    let mut values = records.values().cloned().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.host
            .cmp(&right.host)
            .then(left.port.cmp(&right.port))
            .then(left.fingerprint.cmp(&right.fingerprint))
    });
    values
}

async fn persist_records(path: &Path, records: Vec<TrustedSshHost>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }

    let contents = serde_json::to_vec_pretty(&TrustedSshHostsFile { records })
        .map_err(|error| error.to_string())?;
    tokio::fs::write(path, contents)
        .await
        .map_err(|error| error.to_string())
}

fn record_key(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{
        HostTrustConfirmation, HostTrustMismatch, HostTrustPrompt, SshTrustStore, TrustCheck,
    };

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("noverterm-trust-{}.json", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn trust_store_requires_first_use_then_persists_confirmation() {
        let path = temp_path();
        let store = SshTrustStore::new(path.clone()).expect("trust store should initialize");

        let first_use = store
            .evaluate("example.com", 22, "ssh-ed25519", "SHA256:first-fingerprint")
            .await;
        assert_eq!(
            first_use,
            TrustCheck::TrustRequired(HostTrustPrompt {
                host: "example.com".to_string(),
                port: 22,
                algorithm: "ssh-ed25519".to_string(),
                fingerprint: "SHA256:first-fingerprint".to_string(),
            })
        );

        store
            .confirm(HostTrustConfirmation {
                host: "example.com".to_string(),
                port: 22,
                algorithm: "ssh-ed25519".to_string(),
                fingerprint: "SHA256:first-fingerprint".to_string(),
            })
            .await
            .expect("confirmation should persist");

        let reloaded =
            SshTrustStore::new(path.clone()).expect("reloaded trust store should initialize");
        let trusted = reloaded
            .evaluate("example.com", 22, "ssh-ed25519", "SHA256:first-fingerprint")
            .await;
        assert_eq!(trusted, TrustCheck::Trusted);

        std::fs::remove_file(path).expect("temp trust file should be removable");
    }

    #[tokio::test]
    async fn trust_store_blocks_fingerprint_mismatch() {
        let path = temp_path();
        let store = SshTrustStore::new(path.clone()).expect("trust store should initialize");

        store
            .confirm(HostTrustConfirmation {
                host: "example.com".to_string(),
                port: 22,
                algorithm: "ssh-ed25519".to_string(),
                fingerprint: "SHA256:expected".to_string(),
            })
            .await
            .expect("confirmation should persist");

        let mismatch = store
            .evaluate("example.com", 22, "ssh-ed25519", "SHA256:presented")
            .await;
        assert_eq!(
            mismatch,
            TrustCheck::TrustMismatch(HostTrustMismatch {
                host: "example.com".to_string(),
                port: 22,
                expected_algorithm: "ssh-ed25519".to_string(),
                expected_fingerprint: "SHA256:expected".to_string(),
                presented_algorithm: "ssh-ed25519".to_string(),
                presented_fingerprint: "SHA256:presented".to_string(),
            })
        );

        std::fs::remove_file(path).expect("temp trust file should be removable");
    }
}
