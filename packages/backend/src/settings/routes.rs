use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use shared::Setting;

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;

use super::repository::{self, RepositoryError};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_settings).post(create_setting))
        .route(
            "/{key}",
            get(get_setting).put(update_setting).delete(delete_setting),
        )
}

async fn list_settings(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<Setting>>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let settings = repository::list(pool, authenticated_user.user_id)
        .await
        .map_err(into_http_error)?;

    Ok(Json(settings.into_iter().map(to_setting).collect()))
}

async fn get_setting(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(key): Path<String>,
) -> Result<Json<Setting>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let setting = repository::get(pool, authenticated_user.user_id, key)
        .await
        .map_err(into_http_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "setting not found".to_string()))?;

    Ok(Json(to_setting(setting)))
}

async fn create_setting(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(setting): Json<Setting>,
) -> Result<(StatusCode, Json<Setting>), (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let setting = repository::create(pool, authenticated_user.user_id, setting.key, setting.value)
        .await
        .map_err(into_http_error)?;

    Ok((StatusCode::CREATED, Json(to_setting(setting))))
}

async fn update_setting(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(key): Path<String>,
    Json(setting): Json<Setting>,
) -> Result<Json<Setting>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let setting = repository::update(pool, authenticated_user.user_id, key, setting.value)
        .await
        .map_err(into_http_error)?;

    Ok(Json(to_setting(setting)))
}

async fn delete_setting(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(key): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let deleted = repository::delete(pool, authenticated_user.user_id, key)
        .await
        .map_err(into_http_error)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "setting not found".to_string()))
    }
}

fn to_setting(setting: orm::models::UserSetting) -> Setting {
    Setting {
        key: setting.key,
        value: setting.value,
    }
}

fn into_http_error(error: RepositoryError) -> (StatusCode, String) {
    match error {
        RepositoryError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        RepositoryError::Conflict(message) => (StatusCode::CONFLICT, message),
        RepositoryError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

fn internal_error(message: String) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, message)
}
