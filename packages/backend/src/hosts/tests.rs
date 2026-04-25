use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::test_support::{
    authorized_empty_request, authorized_json_request, build_test_app, login_access_token,
    response_json, unique_name,
};

#[tokio::test]
async fn host_routes_are_owner_scoped_and_redact_password_fields() {
    let alice = unique_name("alice");
    let bob = unique_name("bob");
    let password = "wonderland";
    let app = build_test_app();

    app.clone()
        .oneshot(
            Request::post("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{alice}","password":"{password}"}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("register should succeed");

    app.clone()
        .oneshot(
            Request::post("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{bob}","password":"{password}"}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("register should succeed");

    let alice_token = login_access_token(app.clone(), &alice, password).await;
    let bob_token = login_access_token(app.clone(), &bob, password).await;

    let alice_create = authorized_json_request(
        app.clone(),
        Method::POST,
        "/bootstrap/hosts",
        &alice_token,
        json!({
            "name": "Alice host",
            "host": "alice.example.com",
            "port": 22,
            "username": "deploy",
            "auth_mode": "password",
            "ssh_key_id": null,
            "encrypted_password": "encrypted-alice-password"
        }),
    )
    .await;
    assert_eq!(alice_create.status(), StatusCode::CREATED);
    let alice_host = response_json(alice_create).await;
    assert!(alice_host.get("encrypted_password").is_none());
    let alice_host_id = alice_host["id"].as_str().expect("host id should exist");

    let bob_create = authorized_json_request(
        app.clone(),
        Method::POST,
        "/bootstrap/hosts",
        &bob_token,
        json!({
            "name": "Bob host",
            "host": "bob.example.com",
            "port": 2222,
            "username": "ops",
            "auth_mode": "password",
            "ssh_key_id": null,
            "encrypted_password": "encrypted-bob-password"
        }),
    )
    .await;
    assert_eq!(bob_create.status(), StatusCode::CREATED);
    let bob_host = response_json(bob_create).await;
    let bob_host_id = bob_host["id"].as_str().expect("host id should exist");

    let alice_list =
        authorized_empty_request(app.clone(), Method::GET, "/bootstrap/hosts", &alice_token).await;
    assert_eq!(alice_list.status(), StatusCode::OK);
    let alice_list = response_json(alice_list).await;
    let alice_hosts = alice_list
        .as_array()
        .expect("hosts list should be an array");
    assert!(alice_hosts.iter().any(|host| host["id"] == alice_host_id));
    assert!(!alice_hosts.iter().any(|host| host["id"] == bob_host_id));
    assert!(alice_hosts
        .iter()
        .all(|host| host.get("encrypted_password").is_none()));

    let bob_get_alice = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/bootstrap/hosts/{alice_host_id}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_get_alice.status(), StatusCode::NOT_FOUND);

    let alice_update = authorized_json_request(
        app.clone(),
        Method::PUT,
        &format!("/bootstrap/hosts/{alice_host_id}"),
        &alice_token,
        json!({
            "name": "Alice host updated",
            "host": "alice-updated.example.com",
            "port": 2200,
            "username": "root",
            "auth_mode": "password",
            "ssh_key_id": null,
            "encrypted_password": "encrypted-alice-password-v2"
        }),
    )
    .await;
    assert_eq!(alice_update.status(), StatusCode::OK);
    let alice_update = response_json(alice_update).await;
    assert_eq!(alice_update["host"], "alice-updated.example.com");
    assert!(alice_update.get("encrypted_password").is_none());

    let bob_delete_alice = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/bootstrap/hosts/{alice_host_id}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_delete_alice.status(), StatusCode::NOT_FOUND);

    let alice_delete = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/bootstrap/hosts/{alice_host_id}"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_delete.status(), StatusCode::NO_CONTENT);
}
