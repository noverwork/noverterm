use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::bootstrap::{build_test_router, test_app_state};

use super::{router, AuthConfig, AuthService};

fn auth_service() -> AuthService {
    AuthService::new(AuthConfig::new(
        [("alice".to_string(), "wonderland".to_string())],
        "backend-test-secret".to_string(),
    ))
}

#[tokio::test]
async fn login_refresh_and_logout_flow_rotates_refresh_tokens() {
    let auth_service = auth_service();
    let app = router().with_state(test_app_state(auth_service.clone()));

    let login_response = app
        .clone()
        .oneshot(
            Request::post("/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"username":"alice","password":"wonderland"}"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("login request should succeed");
    assert_eq!(login_response.status(), StatusCode::OK);
    let body = login_response
        .into_body()
        .collect()
        .await
        .expect("login body should collect")
        .to_bytes();
    let login: serde_json::Value =
        serde_json::from_slice(&body).expect("login body should deserialize");
    let refresh_token = login["refresh_token"]
        .as_str()
        .expect("refresh token should exist");

    let refresh_response = app
        .clone()
        .oneshot(
            Request::post("/refresh")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"refresh_token":"{refresh_token}"}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("refresh request should succeed");
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let body = refresh_response
        .into_body()
        .collect()
        .await
        .expect("refresh body should collect")
        .to_bytes();
    let refreshed: serde_json::Value =
        serde_json::from_slice(&body).expect("refresh body should deserialize");
    let rotated_refresh_token = refreshed["refresh_token"]
        .as_str()
        .expect("rotated refresh token should exist");
    assert_ne!(refresh_token, rotated_refresh_token);

    let reuse_response = app
        .clone()
        .oneshot(
            Request::post("/refresh")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"refresh_token":"{refresh_token}"}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("reused refresh request should succeed");
    assert_eq!(reuse_response.status(), StatusCode::UNAUTHORIZED);

    let logout_response = app
        .oneshot(
            Request::post("/logout")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"refresh_token":"{rotated_refresh_token}"}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("logout request should succeed");
    assert_eq!(logout_response.status(), StatusCode::NO_CONTENT);
    assert_eq!(auth_service.active_session_count_for("alice").await, 0);
}

#[tokio::test]
async fn protected_bootstrap_route_requires_valid_access_token() {
    let auth_service = auth_service();
    let login = auth_service
        .login(super::service::LoginRequest {
            username: "alice".to_string(),
            password: "wonderland".to_string(),
        })
        .await
        .expect("login should succeed");
    let app = build_test_router(auth_service);

    let unauthorized = app
        .clone()
        .oneshot(
            Request::get("/bootstrap/smoke")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let authorized = app
        .oneshot(
            Request::get("/bootstrap/smoke")
                .header("authorization", format!("Bearer {}", login.access_token))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(authorized.status(), StatusCode::OK);
}
