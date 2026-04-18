use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::test_support::{
    authorized_json_request, build_test_app, login_access_token, response_json, unique_name,
};

#[tokio::test]
async fn connect_issue_route_requires_ownership_and_returns_scoped_material() {
    let alice = unique_name("alice");
    let bob = unique_name("bob");
    let password = "wonderland";
    let app = build_test_app(&[
        (alice.clone(), password.to_string()),
        (bob.clone(), password.to_string()),
    ]);

    let alice_token = login_access_token(app.clone(), &alice, password).await;
    let bob_token = login_access_token(app.clone(), &bob, password).await;

    let key_response = authorized_json_request(
        app.clone(),
        Method::POST,
        "/bootstrap/keys",
        &alice_token,
        json!({
            "name": "Alice key",
            "kind": "ed25519",
            "fingerprint": "SHA256:alice-key",
            "encrypted_private_key": "alice-private-key",
            "encrypted_passphrase": "alice-passphrase"
        }),
    )
    .await;
    assert_eq!(key_response.status(), StatusCode::CREATED);
    let key = response_json(key_response).await;
    let ssh_key_id = key["id"].as_str().expect("key id should exist");

    let host_response = authorized_json_request(
        app.clone(),
        Method::POST,
        "/bootstrap/hosts",
        &alice_token,
        json!({
            "name": "Alice host",
            "host": "alice.example.com",
            "port": 22,
            "username": "deploy",
            "auth_mode": "publickey",
            "ssh_key_id": ssh_key_id,
            "encrypted_password": null
        }),
    )
    .await;
    assert_eq!(host_response.status(), StatusCode::CREATED);
    let host = response_json(host_response).await;
    let host_id = host["id"].as_str().expect("host id should exist");

    let issue_response = authorized_json_request(
        app.clone(),
        Method::POST,
        &format!("/bootstrap/connect/{host_id}/issue"),
        &alice_token,
        json!({}),
    )
    .await;
    assert_eq!(issue_response.status(), StatusCode::OK);
    let issued = response_json(issue_response).await;
    assert_eq!(issued["host_id"], host_id);
    assert_eq!(issued["host"], "alice.example.com");
    assert_eq!(issued["username"], "deploy");
    assert_eq!(issued["issued_for_username"], alice);
    assert!(issued["issued_for_session_id"].as_str().is_some());
    assert!(issued["issuance_id"].as_str().is_some());
    assert!(issued["expires_at"].as_str().is_some());
    assert_eq!(issued["auth"]["kind"], "public_key");
    assert_eq!(issued["auth"]["private_key"], "alice-private-key");
    assert_eq!(issued["auth"]["passphrase"], "alice-passphrase");
    assert!(issued.get("encrypted_private_key").is_none());

    let forbidden_response = authorized_json_request(
        app,
        Method::POST,
        &format!("/bootstrap/connect/{host_id}/issue"),
        &bob_token,
        json!({}),
    )
    .await;
    assert_eq!(forbidden_response.status(), StatusCode::NOT_FOUND);
}
