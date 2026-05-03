use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::test_support::{
    authorized_empty_request, authorized_json_request, build_test_app, login_access_token,
    response_json, unique_name,
};

#[tokio::test]
async fn host_group_routes_are_owner_scoped() {
    let alice = unique_name("alice");
    let bob = unique_name("bob");
    let password = "wonderland";
    let app = build_test_app();

    for email in [&alice, &bob] {
        app.clone()
            .oneshot(
                Request::post("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","password":"{password}"}}"#
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
        "/api/bootstrap/host-groups",
        &alice_token,
        json!({ "name": " Production " }),
    )
    .await;
    assert_eq!(alice_create.status(), StatusCode::CREATED);
    let alice_group = response_json(alice_create).await;
    assert_eq!(alice_group["name"], "Production");
    let alice_group_id = alice_group["id"].as_str().expect("group id should exist");

    let bob_get_alice = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/api/bootstrap/host-groups/{alice_group_id}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_get_alice.status(), StatusCode::NOT_FOUND);

    let alice_duplicate = authorized_json_request(
        app.clone(),
        Method::POST,
        "/api/bootstrap/host-groups",
        &alice_token,
        json!({ "name": "Production" }),
    )
    .await;
    assert_eq!(alice_duplicate.status(), StatusCode::CONFLICT);

    let alice_host = authorized_json_request(
        app.clone(),
        Method::POST,
        "/api/bootstrap/hosts",
        &alice_token,
        json!({
            "name": "prod",
            "host": "prod.example.com",
            "port": 22,
            "username": "deploy",
            "ssh_key_id": null,
            "encrypted_password": null,
            "group_id": alice_group_id
        }),
    )
    .await;
    assert_eq!(alice_host.status(), StatusCode::CREATED);
    let alice_host = response_json(alice_host).await;
    let alice_host_id = alice_host["id"].as_str().expect("host id should exist");

    let bob_delete_alice = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/api/bootstrap/host-groups/{alice_group_id}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_delete_alice.status(), StatusCode::NOT_FOUND);

    let alice_delete = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/api/bootstrap/host-groups/{alice_group_id}"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_delete.status(), StatusCode::NO_CONTENT);

    let alice_host_after_delete = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/api/bootstrap/hosts/{alice_host_id}"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_host_after_delete.status(), StatusCode::OK);
    let alice_host_after_delete = response_json(alice_host_after_delete).await;
    assert!(alice_host_after_delete["group_id"].is_null());
}
