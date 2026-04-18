use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::Response;

pub use super::service::AuthenticatedUser;
use crate::bootstrap::AppState;

pub async fn require_authenticated_user(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let Some(header_value) = request.headers().get(header::AUTHORIZATION) else {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "missing authorization header".to_string(),
        ));
    };

    let Ok(header_value) = header_value.to_str() else {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "invalid authorization header".to_string(),
        ));
    };

    let Some(token) = header_value.strip_prefix("Bearer ") else {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            "invalid bearer token".to_string(),
        ));
    };

    let authenticated_user = state
        .auth_service
        .authenticate_access_token(token)
        .await
        .map_err(|error| (axum::http::StatusCode::UNAUTHORIZED, error))?;

    request.extensions_mut().insert(authenticated_user);

    Ok(next.run(request).await)
}
