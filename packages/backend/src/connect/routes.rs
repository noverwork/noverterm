use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;

use super::service::{self, ConnectMaterial, ServiceError};

pub fn router() -> Router<AppState> {
    Router::new().route("/{host_id}/issue", post(issue_connect_material))
}

async fn issue_connect_material(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(host_id): Path<String>,
) -> Result<Json<ConnectMaterial>, (StatusCode, String)> {
    service::issue_connect_material(&state, authenticated_user, host_id)
        .await
        .map(Json)
        .map_err(into_http_error)
}

fn into_http_error(error: ServiceError) -> (StatusCode, String) {
    match error {
        ServiceError::NotFound(message) => (StatusCode::NOT_FOUND, message),
        ServiceError::Invalid(message) => (StatusCode::BAD_REQUEST, message),
        ServiceError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}
