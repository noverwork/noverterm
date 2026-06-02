use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use orm::models::HostSnippet;
use shared::{SnippetRecord, SnippetWriteRequest};

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;

use super::repository::{self, CreateSnippetInput, RepositoryError, UpdateSnippetInput};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_snippets).post(create_snippet))
        .route(
            "/{id}",
            get(get_snippet).put(update_snippet).delete(delete_snippet),
        )
}

async fn list_snippets(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<SnippetRecord>>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let snippets = repository::list_all(pool.clone(), owner_id.clone())
        .await
        .map_err(into_http_error)?;

    let mut records = Vec::with_capacity(snippets.len());
    for snippet in snippets {
        records.push(
            to_record(pool.clone(), &owner_id, snippet)
                .await
                .map_err(into_http_error)?,
        );
    }

    Ok(Json(records))
}

async fn get_snippet(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<SnippetRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let snippet = repository::get(pool.clone(), owner_id.clone(), id)
        .await
        .map_err(into_http_error)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "snippet not found".to_string()))?;

    Ok(Json(
        to_record(pool, &owner_id, snippet)
            .await
            .map_err(into_http_error)?,
    ))
}

async fn create_snippet(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<SnippetWriteRequest>,
) -> Result<(StatusCode, Json<SnippetRecord>), (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let snippet = repository::create(
        pool.clone(),
        CreateSnippetInput {
            owner_id: owner_id.clone(),
            host_id: request.host_id,
            title: request.title,
            body: request.body,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok((
        StatusCode::CREATED,
        Json(
            to_record(pool, &owner_id, snippet)
                .await
                .map_err(into_http_error)?,
        ),
    ))
}

async fn update_snippet(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(request): Json<SnippetWriteRequest>,
) -> Result<Json<SnippetRecord>, (StatusCode, String)> {
    let pool = state.require_db_pool().map_err(internal_error)?;
    let owner_id = authenticated_user.user_id;
    let snippet = repository::update(
        pool.clone(),
        UpdateSnippetInput {
            owner_id: owner_id.clone(),
            id,
            host_id: request.host_id,
            title: request.title,
            body: request.body,
        },
    )
    .await
    .map_err(into_http_error)?;

    Ok(Json(
        to_record(pool, &owner_id, snippet)
            .await
            .map_err(into_http_error)?,
    ))
}

async fn delete_snippet(
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
        Err((StatusCode::NOT_FOUND, "snippet not found".to_string()))
    }
}

async fn to_record(
    pool: crate::db::DbPool,
    _owner_id: &str,
    snippet: HostSnippet,
) -> Result<SnippetRecord, RepositoryError> {
    let host_name = repository::get_host_name(pool, snippet.host_id.clone())
        .await?
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(SnippetRecord {
        id: snippet.id,
        host_id: snippet.host_id,
        host_name,
        title: snippet.title,
        body: snippet.body,
    })
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
