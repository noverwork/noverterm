use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use diesel::r2d2::{self, ConnectionManager};
use diesel::Connection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use crate::auth::{AuthConfig, AuthService};
use crate::bootstrap::build_test_router;
use crate::config::env_value;
use crate::db::DbPool;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrator/migrations");

static DATABASE_READY: OnceLock<()> = OnceLock::new();

fn database_url() -> String {
    env_value("DATABASE_URL")
        .unwrap_or_else(|| panic!("DATABASE_URL must be set in environment or backend env file"))
}

pub fn test_db_pool() -> DbPool {
    let database_url = database_url();

    DATABASE_READY.get_or_init(|| {
        let mut connection = diesel::pg::PgConnection::establish(&database_url)
            .unwrap_or_else(|error| panic!("failed to connect to test database: {error}"));
        connection
            .run_pending_migrations(MIGRATIONS)
            .unwrap_or_else(|error| panic!("failed to run test migrations: {error}"));
    });

    let manager = ConnectionManager::<diesel::pg::PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .unwrap_or_else(|error| panic!("failed to create test database pool: {error}"))
}

pub fn unique_name(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

pub fn build_test_app() -> Router {
    let pool = test_db_pool();
    let auth_service = AuthService::new(
        AuthConfig::new("backend-test-secret".to_string()),
        pool.clone(),
    );

    build_test_router(auth_service, pool)
}

pub async fn login_access_token(app: Router, username: &str, password: &str) -> String {
    let response = app
        .oneshot(
            Request::post("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{username}","password":"{password}"}}"#
                )))
                .expect("login request should build"),
        )
        .await
        .expect("login request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("login body should collect")
        .to_bytes();
    let value: serde_json::Value =
        serde_json::from_slice(&body).expect("login body should deserialize");

    value["access_token"]
        .as_str()
        .expect("access token should exist")
        .to_string()
}

pub async fn authorized_json_request(
    app: Router,
    method: Method,
    path: &str,
    access_token: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {access_token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&body).expect("request body should serialize"),
            ))
            .expect("authorized json request should build"),
    )
    .await
    .expect("authorized json request should succeed")
}

pub async fn authorized_empty_request(
    app: Router,
    method: Method,
    path: &str,
    access_token: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {access_token}"))
            .body(Body::empty())
            .expect("authorized request should build"),
    )
    .await
    .expect("authorized request should succeed")
}

pub async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should collect")
        .to_bytes();

    serde_json::from_slice(&body).expect("response body should deserialize")
}
