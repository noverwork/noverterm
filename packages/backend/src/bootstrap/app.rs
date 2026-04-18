use axum::extract::Extension;
use axum::middleware;
use axum::{routing::get, Json, Router};
use serde::Serialize;

use super::AppState;
use crate::auth::AuthenticatedUser;

pub fn build_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/smoke", get(bootstrap_smoke))
        .nest("/hosts", crate::hosts::router())
        .nest("/keys", crate::keys::router())
        .nest("/connect", crate::connect::router())
        .nest("/settings", crate::settings::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_authenticated_user,
        ));

    Router::new()
        .route("/", get(crate::healthcheck))
        .nest("/auth", crate::auth::router())
        .nest("/bootstrap", protected_routes)
        .with_state(state)
}

#[cfg(test)]
pub fn build_test_router(auth_service: crate::auth::AuthService) -> Router {
    build_router(super::test_app_state(auth_service))
}

#[cfg(test)]
pub fn build_test_router_with_db(
    auth_service: crate::auth::AuthService,
    db_pool: crate::db::DbPool,
) -> Router {
    build_router(super::test_app_state_with_db(auth_service, db_pool))
}

#[derive(Debug, Serialize)]
struct BootstrapSmokeResponse {
    message: String,
    username: String,
}

async fn bootstrap_smoke(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Json<BootstrapSmokeResponse> {
    Json(BootstrapSmokeResponse {
        message: format!("bootstrap ready for {}", authenticated_user.username),
        username: authenticated_user.username,
    })
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::auth::{AuthConfig, AuthService};

    #[tokio::test]
    async fn healthcheck_returns_backend_running_message() {
        assert_eq!(crate::healthcheck().await, "Backend running");
    }

    #[tokio::test]
    async fn router_exposes_healthcheck() {
        let app = super::build_test_router(AuthService::new(AuthConfig::new(
            [("alice".to_string(), "wonderland".to_string())],
            "backend-test-secret".to_string(),
        )));

        let response = app
            .oneshot(
                Request::get("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("healthcheck request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
