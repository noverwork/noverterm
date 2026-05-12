use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use shared::{HostGroupRecord, HostGroupWriteRequest};

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;

use super::repository::{self, CreateHostGroupInput, RepositoryError, UpdateHostGroupInput};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_host_groups).post(create_host_group))
        .route(
            "/{id}",
            get(get_host_group)
                .put(update_host_group)
                .delete(delete_host_group),
        )
}

async fn list_host_groups(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<HostGroupRecord>>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let groups = repository::list(pool, authenticated_user.user_id)
        .await
        .map_err(into_http_error)?;

    Ok(Json(groups.into_iter().map(to_record).collect()))
}

async fn get_host_group(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<HostGroupRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let groups = repository::list(pool, authenticated_user.user_id)
        .await
        .map_err(into_http_error)?;
    let group = groups
        .into_iter()
        .find(|candidate| candidate.id == id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "host group not found".to_string()))?;

    Ok(Json(to_record(group)))
}

async fn create_host_group(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<HostGroupWriteRequest>,
) -> Result<(StatusCode, Json<HostGroupRecord>), (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let group = repository::create(
        pool,
        CreateHostGroupInput {
            owner_id: authenticated_user.user_id,
            name: request.name,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok((StatusCode::CREATED, Json(to_record(group))))
}

async fn update_host_group(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<HostGroupWriteRequest>,
) -> Result<Json<HostGroupRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let group = repository::update(
        pool,
        UpdateHostGroupInput {
            owner_id: authenticated_user.user_id,
            id,
            name: request.name,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok(Json(to_record(group)))
}

async fn delete_host_group(
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
        Err((StatusCode::NOT_FOUND, "host group not found".to_string()))
    }
}

fn to_record(group: orm::models::HostGroup) -> HostGroupRecord {
    HostGroupRecord {
        id: group.id,
        name: group.name,
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
