use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use shared::SshHostRecord;

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;

use super::repository::{self, CreateHostInput, RepositoryError, UpdateHostInput};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_hosts).post(create_host))
        .route("/{id}", get(get_host).put(update_host).delete(delete_host))
}

#[derive(Debug, Deserialize)]
struct HostWriteRequest {
    name: String,
    host: String,
    port: i32,
    username: String,
    auth_mode: String,
    ssh_key_id: Option<String>,
    encrypted_password: Option<String>,
}

async fn list_hosts(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<SshHostRecord>>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let hosts = repository::list(pool, authenticated_user.user_id)
        .await
        .map_err(into_http_error)?;

    Ok(Json(hosts.into_iter().map(to_record).collect()))
}

async fn get_host(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<SshHostRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let host = repository::get(pool, authenticated_user.user_id, id)
        .await
        .map_err(into_http_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "host not found".to_string()))?;

    Ok(Json(to_record(host)))
}

async fn create_host(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<HostWriteRequest>,
) -> Result<(StatusCode, Json<SshHostRecord>), (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let host = repository::create(
        pool,
        CreateHostInput {
            owner_id: authenticated_user.user_id,
            name: request.name,
            host: request.host,
            port: request.port,
            username: request.username,
            auth_mode: request.auth_mode,
            ssh_key_id: request.ssh_key_id,
            encrypted_password: request.encrypted_password,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok((StatusCode::CREATED, Json(to_record(host))))
}

async fn update_host(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<HostWriteRequest>,
) -> Result<Json<SshHostRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let host = repository::update(
        pool,
        UpdateHostInput {
            owner_id: authenticated_user.user_id,
            id,
            name: request.name,
            host: request.host,
            port: request.port,
            username: request.username,
            auth_mode: request.auth_mode,
            ssh_key_id: request.ssh_key_id,
            encrypted_password: request.encrypted_password,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok(Json(to_record(host)))
}

async fn delete_host(
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
        Err((StatusCode::NOT_FOUND, "host not found".to_string()))
    }
}

fn to_record(host: orm::models::SshHost) -> SshHostRecord {
    SshHostRecord {
        id: host.id,
        name: host.name,
        host: host.host,
        port: host.port,
        username: host.username,
        auth_mode: host.auth_mode,
        ssh_key_id: host.ssh_key_id,
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
