use axum::{routing::post, Json, Router};

use crate::bootstrap::AppState;

use super::service::{AuthResponse, LoginRequest, LogoutRequest, RefreshRequest};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}

async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    state
        .auth_service
        .login(request)
        .await
        .map(Json)
        .map_err(|error| (axum::http::StatusCode::UNAUTHORIZED, error))
}

async fn refresh(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    state
        .auth_service
        .refresh(request)
        .await
        .map(Json)
        .map_err(|error| (axum::http::StatusCode::UNAUTHORIZED, error))
}

async fn logout(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<LogoutRequest>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .auth_service
        .logout(request)
        .await
        .map(|_| axum::http::StatusCode::NO_CONTENT)
        .map_err(|error| (axum::http::StatusCode::BAD_REQUEST, error))
}
