use axum::extract::Extension;
use axum::middleware;
use axum::response::Html;
use axum::{routing::get, Json, Router};
use serde::Serialize;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use super::AppState;
use crate::auth::AuthenticatedUser;

pub fn build_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/smoke", get(bootstrap_smoke))
        .nest("/host-groups", crate::host_groups::router())
        .nest("/hosts", crate::hosts::router())
        .nest("/keys", crate::keys::router())
        .nest("/settings", crate::settings::router())
        .nest("/snippets", crate::snippets::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_authenticated_user,
        ));

    let api_routes = Router::new()
        .route("/", get(crate::healthcheck))
        .nest("/auth", crate::auth::router())
        .merge(protected_routes);

    Router::new()
        .route("/reset-password", get(password_reset_page))
        .nest("/api", api_routes)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}

async fn password_reset_page() -> Html<&'static str> {
    Html(PASSWORD_RESET_PAGE)
}

const PASSWORD_RESET_PAGE: &str = r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Reset your Noverterm password</title>
    <style>
      :root {
        color-scheme: dark;
        font-family:
          Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        background: #020617;
        color: #e2e8f0;
      }

      body {
        min-height: 100vh;
        margin: 0;
        display: grid;
        place-items: center;
        background:
          radial-gradient(circle at top left, rgba(56, 189, 248, 0.18), transparent 30rem),
          radial-gradient(circle at bottom right, rgba(34, 197, 94, 0.16), transparent 28rem),
          #020617;
      }

      main {
        width: min(100% - 2rem, 28rem);
        border: 1px solid rgba(148, 163, 184, 0.25);
        border-radius: 1.5rem;
        padding: 2rem;
        background: rgba(15, 23, 42, 0.86);
        box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
      }

      h1 {
        margin: 0 0 0.5rem;
        font-size: 1.75rem;
      }

      p {
        color: #94a3b8;
        line-height: 1.6;
      }

      label {
        display: block;
        margin: 1rem 0 0.4rem;
        font-size: 0.9rem;
        font-weight: 600;
      }

      input {
        box-sizing: border-box;
        width: 100%;
        border: 1px solid rgba(148, 163, 184, 0.35);
        border-radius: 0.9rem;
        padding: 0.85rem 1rem;
        background: rgba(15, 23, 42, 0.9);
        color: #e2e8f0;
        font: inherit;
      }

      button {
        width: 100%;
        margin-top: 1.25rem;
        border: 0;
        border-radius: 0.9rem;
        padding: 0.9rem 1rem;
        background: linear-gradient(135deg, #38bdf8, #22c55e);
        color: #020617;
        font: inherit;
        font-weight: 800;
        cursor: pointer;
      }

      button:disabled {
        cursor: wait;
        opacity: 0.65;
      }

      .message {
        min-height: 1.5rem;
        margin-top: 1rem;
        font-size: 0.95rem;
      }

      .error {
        color: #fca5a5;
      }

      .success {
        color: #86efac;
      }
    </style>
  </head>
  <body>
    <main>
      <h1>Reset your password</h1>
      <p>Choose a new password for your Noverterm account.</p>
      <form id="reset-form">
        <label for="password">New password</label>
        <input id="password" name="password" type="password" autocomplete="new-password" minlength="6" required />
        <label for="confirm-password">Confirm password</label>
        <input id="confirm-password" name="confirm-password" type="password" autocomplete="new-password" minlength="6" required />
        <button id="submit-button" type="submit">Reset password</button>
        <p id="message" class="message" role="status"></p>
      </form>
    </main>
    <script>
      const form = document.querySelector("#reset-form");
      const passwordInput = document.querySelector("#password");
      const confirmPasswordInput = document.querySelector("#confirm-password");
      const submitButton = document.querySelector("#submit-button");
      const message = document.querySelector("#message");
      const token = new URLSearchParams(window.location.search).get("token");

      function setMessage(text, kind) {
        message.textContent = text;
        message.className = `message ${kind}`;
      }

      if (!token) {
        submitButton.disabled = true;
        setMessage("This reset link is missing a token. Please request a new password reset email.", "error");
      }

      form.addEventListener("submit", async (event) => {
        event.preventDefault();
        if (!token) {
          return;
        }

        const password = passwordInput.value;
        if (password.length < 6) {
          setMessage("Password must be at least 6 characters.", "error");
          return;
        }

        if (password !== confirmPasswordInput.value) {
          setMessage("Passwords do not match.", "error");
          return;
        }

        submitButton.disabled = true;
        setMessage("Resetting password...", "");

        try {
          const response = await fetch("/api/auth/reset-password", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ token, password }),
          });

          if (!response.ok) {
            const errorText = await response.text();
            throw new Error(errorText || "Failed to reset password.");
          }

          form.reset();
          setMessage("Password reset successfully. You can now sign in from Noverterm.", "success");
        } catch (error) {
          setMessage(error instanceof Error ? error.message : "Failed to reset password.", "error");
          submitButton.disabled = false;
        }
      });
    </script>
  </body>
</html>"##;

#[cfg(test)]
pub fn build_test_router(
    auth_service: crate::auth::AuthService,
    db_pool: crate::db::DbPool,
) -> Router {
    build_router(super::test_app_state(auth_service, db_pool))
}

#[derive(Debug, Serialize)]
struct BootstrapSmokeResponse {
    message: String,
    email: String,
}

async fn bootstrap_smoke(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Json<BootstrapSmokeResponse> {
    Json(BootstrapSmokeResponse {
        message: format!("bootstrap ready for {}", authenticated_user.email),
        email: authenticated_user.email,
    })
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::auth::{AuthConfig, AuthService};

    #[tokio::test]
    async fn healthcheck_returns_backend_running_message() {
        assert_eq!(crate::healthcheck().await, "Backend running");
    }

    #[tokio::test]
    async fn router_exposes_healthcheck() {
        let pool = crate::test_support::test_db_pool();
        let auth_service = AuthService::new(
            AuthConfig::new("backend-test-secret".to_string()),
            pool.clone(),
        );
        let app = super::build_test_router(auth_service, pool);

        let response = app
            .oneshot(
                Request::get("/api")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("healthcheck request should succeed");

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn password_reset_page_posts_to_reset_api() {
        let axum::response::Html(page) = super::password_reset_page().await;

        assert!(page.contains("Reset your password"));
        assert!(page.contains("/api/auth/reset-password"));
        assert!(page.contains("new URLSearchParams(window.location.search).get(\"token\")"));
    }
}
