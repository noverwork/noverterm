use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use shared::SshKeyRecord;

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;

use super::repository::{self, CreateKeyInput, RepositoryError, UpdateKeyInput};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_keys).post(create_key))
        .route("/{id}", get(get_key).put(update_key).delete(delete_key))
}

#[derive(Debug, Deserialize)]
struct KeyWriteRequest {
    name: String,
    kind: String,
    fingerprint: Option<String>,
    encrypted_private_key: String,
    encrypted_passphrase: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeyUpdateRequest {
    name: String,
    kind: String,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(default)]
    encrypted_private_key: Option<String>,
    #[serde(default)]
    encrypted_passphrase: Option<String>,
}

async fn list_keys(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<SshKeyRecord>>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let keys = repository::list(pool, authenticated_user.user_id)
        .await
        .map_err(into_http_error)?;

    Ok(Json(keys.into_iter().map(to_record).collect()))
}

async fn get_key(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<SshKeyRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let key = repository::get(pool, authenticated_user.user_id, id)
        .await
        .map_err(into_http_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "ssh key not found".to_string()))?;

    Ok(Json(to_record(key)))
}

async fn create_key(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<KeyWriteRequest>,
) -> Result<(StatusCode, Json<SshKeyRecord>), (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let key = repository::create(
        pool,
        CreateKeyInput {
            owner_id: authenticated_user.user_id,
            name: request.name,
            kind: request.kind,
            fingerprint: request.fingerprint,
            encrypted_private_key: request.encrypted_private_key,
            encrypted_passphrase: request.encrypted_passphrase,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok((StatusCode::CREATED, Json(to_record(key))))
}

async fn update_key(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<KeyUpdateRequest>,
) -> Result<Json<SshKeyRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let key = repository::update(
        pool,
        UpdateKeyInput {
            owner_id: authenticated_user.user_id,
            id,
            name: request.name,
            kind: request.kind,
            fingerprint: request.fingerprint,
            encrypted_private_key: request.encrypted_private_key,
            encrypted_passphrase: request.encrypted_passphrase,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok(Json(to_record(key)))
}

async fn delete_key(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let deleted = repository::delete(pool, authenticated_user.user_id, id)
        .await
        .map_err(into_http_error)?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "ssh key not found".to_string()))
    }
}

fn to_record(key: orm::models::SshKey) -> SshKeyRecord {
    SshKeyRecord {
        id: key.id,
        name: key.name,
        kind: key.kind,
        fingerprint: key.fingerprint,
    }
}

fn into_http_error(error: RepositoryError) -> (StatusCode, String) {
    match error {
        RepositoryError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        RepositoryError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

fn internal_error(message: String) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, message)
}
