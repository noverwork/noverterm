use tempfile::TempDir;

use super::{HostTrustConfirmation, HostTrustMismatch, HostTrustPrompt, SshTrustStore, TrustCheck};
use crate::store::test_pool;

fn test_store() -> (SshTrustStore, TempDir) {
    let directory = tempfile::tempdir().expect("temp dir");
    let store = SshTrustStore::new(test_pool(&directory));
    (store, directory)
}

#[tokio::test]
async fn trust_store_requires_first_use_then_persists_confirmation() {
    let (store, _directory) = test_store();

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

    let trusted = store
        .evaluate("example.com", 22, "ssh-ed25519", "SHA256:first-fingerprint")
        .await;
    assert_eq!(trusted, TrustCheck::Trusted);

    let listed = store.list().await.expect("list known hosts");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].host, "example.com");
    assert_eq!(listed[0].port, 22);
}

#[tokio::test]
async fn trust_store_blocks_fingerprint_mismatch() {
    let (store, _directory) = test_store();

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
}

#[tokio::test]
async fn removing_a_known_host_reports_when_there_was_nothing_to_remove() {
    let (store, _directory) = test_store();

    let error = store
        .remove("example.com", 22)
        .await
        .expect_err("removing an unknown host should fail");
    assert!(
        error.contains("example.com:22"),
        "unexpected error: {error}"
    );

    store
        .confirm(HostTrustConfirmation {
            host: "example.com".to_string(),
            port: 22,
            algorithm: "ssh-ed25519".to_string(),
            fingerprint: "SHA256:expected".to_string(),
        })
        .await
        .expect("confirmation should persist");

    store.remove("example.com", 22).await.expect("remove host");
    assert!(store.list().await.expect("list known hosts").is_empty());
}
