use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use super::backend_client::BackendClient;
use super::session::{AuthBootstrapStatus, AuthManager};
use super::token_store::MemoryTokenStore;

#[tokio::test]
async fn restore_refreshes_expired_access_token_and_keeps_session() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener
        .local_addr()
        .expect("listener should have an address");
    let server = tokio::spawn(async move {
        axum::serve(listener, test_auth_server())
            .await
            .expect("test auth server should run");
    });

    let manager = AuthManager::new(
        BackendClient::new(format!("http://{}", address)),
        MemoryTokenStore::default(),
    );

    let login_status = manager
        .login("alice".to_string(), "wonderland".to_string())
        .await
        .expect("login should succeed");
    assert_eq!(
        login_status,
        AuthBootstrapStatus {
            email: "alice".to_string(),
            bootstrap_message: "bootstrap ready".to_string(),
        }
    );

    let restored_status = manager
        .restore()
        .await
        .expect("restore should succeed")
        .expect("restore should return a session");
    assert_eq!(restored_status.email, "alice");

    manager.logout().await.expect("logout should succeed");
    let restored_after_logout = manager
        .restore()
        .await
        .expect("restore should still succeed");
    assert!(restored_after_logout.is_none());

    server.abort();
}

#[tokio::test]
async fn connect_material_request_refreshes_access_token_when_needed() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener
        .local_addr()
        .expect("listener should have an address");
    let server = tokio::spawn(async move {
        axum::serve(listener, test_auth_server())
            .await
            .expect("test auth server should run");
    });

    let manager = AuthManager::new(
        BackendClient::new(format!("http://{}", address)),
        MemoryTokenStore::default(),
    );

    manager
        .login("alice".to_string(), "wonderland".to_string())
        .await
        .expect("login should succeed");

    let material = manager
        .issue_connect_material("host-123".to_string())
        .await
        .expect("connect material should refresh and load");
    assert_eq!(material.host_id, "host-123");
    assert_eq!(material.host, "example.com");
    assert_eq!(material.port, 22);
    assert_eq!(material.username, "deploy");

    server.abort();
}

#[tokio::test]
async fn bootstrap_metadata_loads_settings_hosts_and_keys() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener
        .local_addr()
        .expect("listener should have an address");
    let server = tokio::spawn(async move {
        axum::serve(listener, test_auth_server())
            .await
            .expect("test auth server should run");
    });

    let manager = AuthManager::new(
        BackendClient::new(format!("http://{}", address)),
        MemoryTokenStore::default(),
    );

    manager
        .login("alice".to_string(), "wonderland".to_string())
        .await
        .expect("login should succeed");

    let metadata = manager
        .load_bootstrap_metadata()
        .await
        .expect("metadata should load");

    assert_eq!(metadata.settings.len(), 1);
    assert_eq!(metadata.settings[0].key, "noverterm-config");
    assert_eq!(metadata.hosts.len(), 1);
    assert_eq!(metadata.hosts[0].name, "prod-server");
    assert_eq!(metadata.keys.len(), 1);
    assert_eq!(metadata.keys[0].name, "deploy-key");

    server.abort();
}

#[tokio::test]
async fn bootstrap_metadata_fails_when_not_authenticated() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener
        .local_addr()
        .expect("listener should have an address");
    let server = tokio::spawn(async move {
        axum::serve(listener, test_auth_server())
            .await
            .expect("test auth server should run");
    });

    let manager = AuthManager::new(
        BackendClient::new(format!("http://{}", address)),
        MemoryTokenStore::default(),
    );

    let result = manager.load_bootstrap_metadata().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not authenticated"));

    server.abort();
}

fn test_auth_server() -> Router {
    Router::new()
        .route("/auth/login", post(login_handler))
        .route("/auth/refresh", post(refresh_handler))
        .route("/auth/logout", post(logout_handler))
        .route("/bootstrap/connect/{host_id}/issue", post(connect_handler))
        .route("/bootstrap/smoke", get(smoke_handler))
        .route("/bootstrap/settings", get(settings_handler))
        .route("/bootstrap/hosts", get(hosts_handler))
        .route("/bootstrap/keys", get(keys_handler))
        .with_state(())
}

async fn login_handler(
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let email = payload["email"].as_str().unwrap_or_default();
    let password = payload["password"].as_str().unwrap_or_default();

    if email == "alice" && password == "wonderland" {
        (
            StatusCode::OK,
            Json(json!({
                "access_token": "expired-access-token",
                "refresh_token": "good-refresh-token",
                "email": "alice"
            })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid credentials" })),
        )
    }
}

async fn refresh_handler(
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let refresh_token = payload["refresh_token"].as_str().unwrap_or_default();

    if refresh_token == "good-refresh-token" {
        (
            StatusCode::OK,
            Json(json!({
                "access_token": "fresh-access-token",
                "refresh_token": "rotated-refresh-token",
                "email": "alice"
            })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid refresh token" })),
        )
    }
}

async fn logout_handler(Json(_payload): Json<serde_json::Value>) -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn smoke_handler(
    State(()): State<()>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if token == "Bearer fresh-access-token" {
        (
            StatusCode::OK,
            Json(json!({ "message": "bootstrap ready", "email": "alice" })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
    }
}

async fn connect_handler(
    State(()): State<()>,
    axum::extract::Path(host_id): axum::extract::Path<String>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if token == "Bearer fresh-access-token" {
        (
            StatusCode::OK,
            Json(json!({
                "issuance_id": "issuance-1",
                "host_id": host_id,
                "host": "example.com",
                "port": 22,
                "username": "deploy",
                "issued_for_username": "alice",
                "issued_for_session_id": "session-123",
                "expires_at": "2026-04-18T12:00:00Z",
                "auth": {
                    "kind": "password",
                    "password": "secret"
                }
            })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
    }
}

async fn settings_handler(
    State(()): State<()>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if token.starts_with("Bearer ") {
        (
            StatusCode::OK,
            Json(json!([
                { "key": "noverterm-config", "value": "{\"terminal\":{\"theme\":\"dark\",\"fontSize\":16}}" }
            ])),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
    }
}

async fn hosts_handler(
    State(()): State<()>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if token.starts_with("Bearer ") {
        (
            StatusCode::OK,
            Json(json!([
                {
                    "id": "host-1",
                    "name": "prod-server",
                    "host": "prod.example.com",
                    "port": 22,
                    "username": "deploy",
                    "auth_mode": "key",
                    "ssh_key_id": "key-1"
                }
            ])),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
    }
}

async fn keys_handler(
    State(()): State<()>,
    headers: HeaderMap,
) -> (StatusCode, Json<serde_json::Value>) {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if token.starts_with("Bearer ") {
        (
            StatusCode::OK,
            Json(json!([
                {
                    "id": "key-1",
                    "name": "deploy-key",
                    "kind": "ed25519",
                    "fingerprint": "SHA256:abc123"
                }
            ])),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
    }
}
