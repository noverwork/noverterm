use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::test_support::{
    authorized_empty_request, authorized_json_request, build_test_app, login_access_token,
    response_json, unique_name,
};

#[tokio::test]
async fn settings_routes_are_owner_scoped() {
    let alice = unique_name("alice");
    let bob = unique_name("bob");
    let password = "wonderland";
    let app = build_test_app();

    for user in [&alice, &bob] {
        app.clone()
            .oneshot(
                Request::post("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"username":"{user}","password":"{password}"}}"#
                    )))
                    .expect("request should build"),
            )
            .await
            .expect("register should succeed");
    }

    let alice_token = login_access_token(app.clone(), &alice, password).await;
    let bob_token = login_access_token(app.clone(), &bob, password).await;
    let alice_setting_key = unique_name("theme");
    let bob_setting_key = unique_name("theme");

    let alice_create = authorized_json_request(
        app.clone(),
        Method::POST,
        "/bootstrap/settings",
        &alice_token,
        json!({
            "key": alice_setting_key,
            "value": "dark"
        }),
    )
    .await;
    assert_eq!(alice_create.status(), StatusCode::CREATED);
    let alice_setting = response_json(alice_create).await;
    let alice_setting_key = alice_setting["key"]
        .as_str()
        .expect("setting key should exist");

    let bob_create = authorized_json_request(
        app.clone(),
        Method::POST,
        "/bootstrap/settings",
        &bob_token,
        json!({
            "key": bob_setting_key,
            "value": "light"
        }),
    )
    .await;
    assert_eq!(bob_create.status(), StatusCode::CREATED);

    let alice_list = authorized_empty_request(
        app.clone(),
        Method::GET,
        "/bootstrap/settings",
        &alice_token,
    )
    .await;
    assert_eq!(alice_list.status(), StatusCode::OK);
    let alice_list = response_json(alice_list).await;
    let alice_settings = alice_list
        .as_array()
        .expect("settings list should be an array");
    assert_eq!(alice_settings.len(), 1);
    assert_eq!(alice_settings[0]["value"], "dark");

    let bob_get_alice = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/bootstrap/settings/{alice_setting_key}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_get_alice.status(), StatusCode::NOT_FOUND);

    let bob_get_own = authorized_empty_request(
        app.clone(),
        Method::GET,
        &format!("/bootstrap/settings/{bob_setting_key}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_get_own.status(), StatusCode::OK);
    let bob_setting = response_json(bob_get_own).await;
    assert_eq!(bob_setting["value"], "light");

    let alice_update = authorized_json_request(
        app.clone(),
        Method::PUT,
        &format!("/bootstrap/settings/{alice_setting_key}"),
        &alice_token,
        json!({
            "key": alice_setting_key,
            "value": "system"
        }),
    )
    .await;
    assert_eq!(alice_update.status(), StatusCode::OK);
    let alice_update = response_json(alice_update).await;
    assert_eq!(alice_update["value"], "system");

    let bob_delete_alice = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/bootstrap/settings/{alice_setting_key}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_delete_alice.status(), StatusCode::NOT_FOUND);

    let alice_delete = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/bootstrap/settings/{alice_setting_key}"),
        &alice_token,
    )
    .await;
    assert_eq!(alice_delete.status(), StatusCode::NO_CONTENT);

    let bob_delete_own = authorized_empty_request(
        app.clone(),
        Method::DELETE,
        &format!("/bootstrap/settings/{bob_setting_key}"),
        &bob_token,
    )
    .await;
    assert_eq!(bob_delete_own.status(), StatusCode::NO_CONTENT);
}
