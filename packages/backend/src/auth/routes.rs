use axum::{routing::post, Json, Router};
use shared::{AuthResponse, ForgotPasswordRequest, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest, ResetPasswordRequest};

use crate::bootstrap::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
}

async fn register(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (axum::http::StatusCode, String)> {
    state
        .auth_service
        .register(request)
        .await
        .map(Json)
        .map_err(|error| (axum::http::StatusCode::CONFLICT, error))
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

async fn forgot_password(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .auth_service
        .request_password_reset(request)
        .await
        .map(|_| axum::http::StatusCode::NO_CONTENT)
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error))
}

async fn reset_password(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .auth_service
        .reset_password(request)
        .await
        .map(|_| axum::http::StatusCode::NO_CONTENT)
        .map_err(|error| (axum::http::StatusCode::BAD_REQUEST, error))
}
