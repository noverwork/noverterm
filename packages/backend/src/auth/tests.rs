use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::auth::{AuthConfig, AuthService};
use crate::test_support::{login_access_token, test_db_pool};

fn auth_service() -> AuthService {
    AuthService::new(
        AuthConfig::new("backend-test-secret".to_string()),
        test_db_pool(),
    )
}

#[tokio::test]
async fn register_and_login_flow_creates_session() {
    let auth_service = auth_service();

    let register_response = auth_service
        .register(super::service::RegisterRequest {
            email: "alice".to_string(),
            password: "wonderland".to_string(),
        })
        .await
        .expect("register should succeed");

    assert_eq!(register_response.email, "alice");
    assert!(!register_response.access_token.is_empty());

    let login_response = auth_service
        .login(super::service::LoginRequest {
            email: "alice".to_string(),
            password: "wonderland".to_string(),
        })
        .await
        .expect("login should succeed");

    assert_eq!(login_response.email, "alice");
}

#[tokio::test]
async fn login_refresh_and_logout_flow_rotates_refresh_tokens() {
    let auth_service = auth_service();

    auth_service
        .register(super::service::RegisterRequest {
            email: "bob".to_string(),
            password: "secret".to_string(),
        })
        .await
        .expect("register should succeed");

    let login_response = auth_service
        .login(super::service::LoginRequest {
            email: "bob".to_string(),
            password: "secret".to_string(),
        })
        .await
        .expect("login request should succeed");

    let refresh_token = login_response.refresh_token;

    let refresh_response = auth_service
        .refresh(super::service::RefreshRequest {
            refresh_token: refresh_token.clone(),
        })
        .await
        .expect("refresh request should succeed");

    let rotated_refresh_token = refresh_response.refresh_token;
    assert_ne!(refresh_token, rotated_refresh_token);

    let reuse_response = auth_service
        .refresh(super::service::RefreshRequest {
            refresh_token: refresh_token.clone(),
        })
        .await;
    assert!(reuse_response.is_err());

    let logout_result = auth_service
        .logout(super::service::LogoutRequest {
            refresh_token: rotated_refresh_token,
        })
        .await;
    assert!(logout_result.is_ok());
    assert_eq!(auth_service.active_session_count_for("bob").await, 0);
}

#[tokio::test]
async fn register_duplicate_username_fails() {
    let auth_service = auth_service();

    auth_service
        .register(super::service::RegisterRequest {
            username: "charlie".to_string(),
            password: "pass1".to_string(),
        })
        .await
        .expect("first register should succeed");

    let result = auth_service
        .register(super::service::RegisterRequest {
            username: "charlie".to_string(),
            password: "pass2".to_string(),
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn protected_bootstrap_route_requires_valid_access_token() {
    let pool = test_db_pool();
    let auth_service = AuthService::new(
        AuthConfig::new("backend-test-secret".to_string()),
        pool.clone(),
    );

    auth_service
        .register(super::service::RegisterRequest {
            email: "dave".to_string(),
            password: "pass".to_string(),
        })
        .await
        .expect("register should succeed");

    let access_token = login_access_token(
        crate::test_support::build_test_app(),
        "dave",
        "pass",
    )
    .await;

    let app = crate::bootstrap::build_test_router(auth_service, pool);

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
                .header("authorization", format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");
    assert_eq!(authorized.status(), StatusCode::OK);
}
