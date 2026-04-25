use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use shared::{SshHostAuthMaterial, SshHostRecord};

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;
use crate::keys::repository as key_repository;

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
    ssh_key_id: Option<String>,
    encrypted_password: Option<String>,
}

async fn list_hosts(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<SshHostRecord>>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let hosts = repository::list(pool.clone(), owner_id.clone())
        .await
        .map_err(into_http_error)?;

    let mut records = Vec::with_capacity(hosts.len());
    for host in hosts {
        records.push(
            to_record(pool.clone(), &owner_id, host)
                .await
                .map_err(into_http_error)?,
        );
    }

    Ok(Json(records))
}

async fn get_host(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<SshHostRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let host = repository::get(pool.clone(), owner_id.clone(), id)
        .await
        .map_err(into_http_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "host not found".to_string()))?;

    Ok(Json(
        to_record(pool, &owner_id, host)
            .await
            .map_err(into_http_error)?,
    ))
}

async fn create_host(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<HostWriteRequest>,
) -> Result<(StatusCode, Json<SshHostRecord>), (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let host = repository::create(
        pool.clone(),
        CreateHostInput {
            owner_id: owner_id.clone(),
            name: request.name,
            host: request.host,
            port: request.port,
            username: request.username,
            ssh_key_id: request.ssh_key_id,
            encrypted_password: request.encrypted_password,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok((
        StatusCode::CREATED,
        Json(
            to_record(pool, &owner_id, host)
                .await
                .map_err(into_http_error)?,
        ),
    ))
}

async fn update_host(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<HostWriteRequest>,
) -> Result<Json<SshHostRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let host = repository::update(
        pool.clone(),
        UpdateHostInput {
            owner_id: owner_id.clone(),
            id,
            name: request.name,
            host: request.host,
            port: request.port,
            username: request.username,
            ssh_key_id: request.ssh_key_id,
            encrypted_password: request.encrypted_password,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok(Json(
        to_record(pool, &owner_id, host)
            .await
            .map_err(into_http_error)?,
    ))
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

async fn to_record(
    pool: crate::db::DbPool,
    owner_id: &str,
    host: orm::models::SshHost,
) -> Result<SshHostRecord, RepositoryError> {
    let auth = host_auth_material(pool, owner_id, &host).await?;

    Ok(SshHostRecord {
        id: host.id,
        name: host.name,
        host: host.host,
        port: host.port,
        username: host.username,
        ssh_key_id: host.ssh_key_id,
        auth,
    })
}

async fn host_auth_material(
    pool: crate::db::DbPool,
    owner_id: &str,
    host: &orm::models::SshHost,
) -> Result<Option<SshHostAuthMaterial>, RepositoryError> {
    let key = match &host.ssh_key_id {
        Some(ssh_key_id) => key_repository::get(pool, owner_id.to_string(), ssh_key_id.clone())
            .await
            .map_err(map_key_error)?,
        None => None,
    };

    Ok(match (key, host.encrypted_password.clone()) {
        (Some(key), Some(password)) => Some(SshHostAuthMaterial::PublicKeyAndPassword {
            private_key: key.encrypted_private_key,
            passphrase: key.encrypted_passphrase,
            password,
        }),
        (Some(key), None) => Some(SshHostAuthMaterial::PublicKey {
            private_key: key.encrypted_private_key,
            passphrase: key.encrypted_passphrase,
        }),
        (None, Some(password)) => Some(SshHostAuthMaterial::Password { password }),
        (None, None) => None,
    })
}

fn map_key_error(error: key_repository::RepositoryError) -> RepositoryError {
    match error {
        key_repository::RepositoryError::NotFound(message) => RepositoryError::NotFound(message),
        key_repository::RepositoryError::Internal(message) => RepositoryError::Internal(message),
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
