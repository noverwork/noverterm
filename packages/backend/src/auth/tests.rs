use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::auth::{AuthConfig, AuthService};
use crate::test_support::{login_access_token, test_db_pool, unique_name};

fn auth_service() -> AuthService {
    AuthService::new(
        AuthConfig::new("backend-test-secret".to_string()),
        test_db_pool(),
    )
}

#[tokio::test]
async fn register_and_login_flow() {
    let auth_service = auth_service();
    let email = unique_name("alice");

    let register_response = auth_service
        .register(super::service::RegisterRequest {
            email: email.clone(),
            password: "wonderland".to_string(),
        })
        .await
        .expect("register should succeed");

    assert_eq!(register_response.email, email);
    assert!(!register_response.access_token.is_empty());
    assert!(!register_response.refresh_token.is_empty());

    let login_response = auth_service
        .login(super::service::LoginRequest {
            email: email.clone(),
            password: "wonderland".to_string(),
        })
        .await
        .expect("login should succeed");

    assert_eq!(login_response.email, email);
}

#[tokio::test]
async fn refresh_returns_new_tokens() {
    let auth_service = auth_service();
    let email = unique_name("bob");

    auth_service
        .register(super::service::RegisterRequest {
            email: email.clone(),
            password: "secret".to_string(),
        })
        .await
        .expect("register should succeed");

    let login_response = auth_service
        .login(super::service::LoginRequest {
            email: email.clone(),
            password: "secret".to_string(),
        })
        .await
        .expect("login should succeed");

    let refresh_response = auth_service
        .refresh(super::service::RefreshRequest {
            refresh_token: login_response.refresh_token,
        })
        .await
        .expect("refresh should succeed");

    assert_eq!(refresh_response.email, email);
    assert!(!refresh_response.access_token.is_empty());
    assert!(!refresh_response.refresh_token.is_empty());
}

#[tokio::test]
async fn invalid_refresh_token_fails() {
    let auth_service = auth_service();

    let result = auth_service
        .refresh(super::service::RefreshRequest {
            refresh_token: "not-a-valid-jwt".to_string(),
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn logout_is_noop() {
    let auth_service = auth_service();

    let result = auth_service
        .logout(super::service::LogoutRequest {
            refresh_token: "any-token".to_string(),
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn register_duplicate_email_fails() {
    let auth_service = auth_service();
    let email = unique_name("charlie");

    auth_service
        .register(super::service::RegisterRequest {
            email: email.clone(),
            password: "pass1".to_string(),
        })
        .await
        .expect("first register should succeed");

    let result = auth_service
        .register(super::service::RegisterRequest {
            email: email.clone(),
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
    let email = unique_name("dave");

    auth_service
        .register(super::service::RegisterRequest {
            email: email.clone(),
            password: "pass".to_string(),
        })
        .await
        .expect("register should succeed");

    let access_token =
        login_access_token(crate::test_support::build_test_app(), &email, "pass").await;

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
