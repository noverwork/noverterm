use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::test_support::{
    authorized_empty_request, authorized_json_request, build_test_app, login_access_token,
    response_json, unique_name,
};

#[tokio::test]
async fn key_routes_are_owner_scoped_and_redact_secret_fields() {
    let alice = unique_name("alice");
    let bob = unique_name("bob");
    let password = "wonderland";
    let app = build_test_app();

    for user in [&alice, &bob] {
        app.clone()
            .oneshot(
                Request::post("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{user}","password":"{password}"}}"#
                    )))
                    .expect("request should build"),
            )
            .await
            .expect("register should succeed");
    }

    let alice_token = login_access_token(app.clone(), &alice, password).await;
    let bob_token = login_access_token(app.clone(), &bob, password).await;

    let alice_create = authorized_json_request(
        app.clone(),
        Method::POST,
        "/api/bootstrap/keys",
        &alice_token,
        json!({
            "name": "Alice key",
            "kind": "ed25519",
            "fingerprint": "SHA256:alice",
            "encrypted_private_key": "encrypted-alice-private-key",
            "encrypted_passphrase": "encrypted-alice-passphrase"
        }),
    )
    .await;
    assert_eq!(alice_create.status(), StatusCode::CREATED);
    let alice_key = response_json(alice_create).await;
    assert!(alice_key.get("encrypted_private_key").is_none());
    assert!(alice_key.get("encrypted_passphrase").is_none());
    let alice_key_id = alice_key["id"].as_str().expect("key id should exist");

    let bob_create = authorized_json_request(
        app.clone(),
        Method::POST,
        "/api/bootstrap/keys",
        &bob_token,
        json!({
            "name": "Bob key",
            "kind": "rsa",
            "fingerprint": "SHA256:bob",
            "encrypted_private_key": "encrypted-bob-private-key",
            "encrypted_passphrase": null
        }),
    )
    .await;
    assert_eq!(bob_create.status(), StatusCode::CREATED);
    let bob_key = response_json(bob_create).await;
    let bob_key_id = bob_key["id"].as_str().expect("key id should exist");

    let alice_list = authorized_empty_request(
        app.clone(),
        Method::GET,
        "/api/bootstrap/keys",
        &alice_token,
    )
    .await;
    assert_eq!(alice_list.status(), StatusCode::OK);
    let alice_list = response_json(alice_list).await;
    let alice_keys = alice_list.as_array().expect("keys list should be an array");
    assert!(alice_keys.iter().any(|key| key["id"] == alice_key_id));
    assert!(!alice_keys.iter().any(|key| key["id"] == bob_key_id));
    assert!(alice_keys
        .iter()
        .all(|key| key.get("encrypted_private_key").is_none()));
    assert!(alice_keys
        .iter()
        .all(|key| key.get("encrypted_passphrase").is_none()));

    let bob_get_alice = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/api/bootstrap/keys/{alice_key_id}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_get_alice.status(), StatusCode::NOT_FOUND);

    let alice_get = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/api/bootstrap/keys/{alice_key_id}"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_get.status(), StatusCode::OK);
    let alice_get = response_json(alice_get).await;
    assert_eq!(alice_get["id"], alice_key_id);
    assert!(alice_get.get("encrypted_private_key").is_none());
    assert!(alice_get.get("encrypted_passphrase").is_none());

    let alice_reveal_secret = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/api/bootstrap/keys/{alice_key_id}/secret"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_reveal_secret.status(), StatusCode::OK);
    let alice_secret = response_json(alice_reveal_secret).await;
    assert_eq!(alice_secret["private_key"], "encrypted-alice-private-key");
    assert_eq!(alice_secret["passphrase"], "encrypted-alice-passphrase");

    let bob_reveal_alice_secret = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/api/bootstrap/keys/{alice_key_id}/secret"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_reveal_alice_secret.status(), StatusCode::NOT_FOUND);

    let alice_update = authorized_json_request(
        app.clone(),
        Method::PUT,
        &format!("/api/bootstrap/keys/{alice_key_id}"),
        &alice_token,
        json!({
            "name": "Alice key updated",
            "kind": "ed25519",
            "fingerprint": "SHA256:alice-updated",
            "encrypted_private_key": "encrypted-alice-private-key-v2",
            "encrypted_passphrase": null
        }),
    )
    .await;
    assert_eq!(alice_update.status(), StatusCode::OK);
    let alice_update = response_json(alice_update).await;
    assert_eq!(alice_update["fingerprint"], "SHA256:alice-updated");
    assert!(alice_update.get("encrypted_private_key").is_none());

    let alice_name_only_update = authorized_json_request(
        app.clone(),
        Method::PUT,
        &format!("/api/bootstrap/keys/{alice_key_id}"),
        &alice_token,
        json!({
            "name": "Alice key renamed",
            "kind": "ed25519"
        }),
    )
    .await;
    assert_eq!(alice_name_only_update.status(), StatusCode::OK);
    let alice_name_only_update = response_json(alice_name_only_update).await;
    assert_eq!(alice_name_only_update["name"], "Alice key renamed");
    assert_eq!(
        alice_name_only_update["fingerprint"],
        "SHA256:alice-updated"
    );
    assert!(alice_name_only_update
        .get("encrypted_private_key")
        .is_none());

    let bob_delete_alice = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/api/bootstrap/keys/{alice_key_id}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_delete_alice.status(), StatusCode::NOT_FOUND);

    let alice_delete = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/api/bootstrap/keys/{alice_key_id}"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_delete.status(), StatusCode::NO_CONTENT);
}
