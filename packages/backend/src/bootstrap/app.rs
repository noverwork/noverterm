use axum::extract::Extension;
use axum::middleware;
use axum::{routing::get, Json, Router};
use serde::Serialize;

use super::AppState;
use crate::auth::AuthenticatedUser;

pub fn build_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/smoke", get(bootstrap_smoke))
        .nest("/host-groups", crate::host_groups::router())
        .nest("/hosts", crate::hosts::router())
        .nest("/keys", crate::keys::router())
        .nest("/settings", crate::settings::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_authenticated_user,
        ));

    Router::new()
        .route("/api", get(crate::healthcheck))
        .nest("/api/auth", crate::auth::router())
        .nest("/api/bootstrap", protected_routes)
        .with_state(state)
}

#[cfg(test)]
pub fn build_test_router(
    auth_service: crate::auth::AuthService,
    db_pool: crate::db::DbPool,
) -> Router {
    build_router(super::test_app_state(auth_service, db_pool))
}

#[derive(Debug, Serialize)]
struct BootstrapSmokeResponse {
    message: String,
    email: String,
}

async fn bootstrap_smoke(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Json<BootstrapSmokeResponse> {
    Json(BootstrapSmokeResponse {
        message: format!("bootstrap ready for {}", authenticated_user.email),
        email: authenticated_user.email,
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
        let pool = crate::test_support::test_db_pool();
        let auth_service = AuthService::new(
            AuthConfig::new("backend-test-secret".to_string()),
            pool.clone(),
        );
        let app = super::build_test_router(auth_service, pool);

        let response = app
            .oneshot(
                Request::get("/api")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("healthcheck request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
