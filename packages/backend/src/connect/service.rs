use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::bootstrap::AppState;
use crate::hosts::repository as host_repository;
use crate::keys::repository as key_repository;

const CONNECT_SECRET_TTL_MINUTES: i64 = 5;

#[derive(Debug)]
pub enum ServiceError {
    NotFound(String),
    Invalid(String),
    Internal(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectMaterial {
    pub issuance_id: String,
    pub host_id: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub issued_for_username: String,
    pub issued_for_session_id: String,
    pub expires_at: DateTime<Utc>,
    pub auth: ConnectAuthMaterial,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConnectAuthMaterial {
    Password {
        password: String,
    },
    PublicKey {
        private_key: String,
        passphrase: Option<String>,
    },
    PublicKeyAndPassword {
        private_key: String,
        passphrase: Option<String>,
        password: String,
    },
}

pub async fn issue_connect_material(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    host_id: String,
) -> Result<ConnectMaterial, ServiceError> {
    let AuthenticatedUser {
        username,
        session_id,
    } = authenticated_user;
    let pool = state.require_db_pool().map_err(ServiceError::Internal)?;
    let host = host_repository::get(pool.clone(), username.clone(), host_id.clone())
        .await
        .map_err(map_host_error)?
        .ok_or_else(|| ServiceError::NotFound("host not found".to_string()))?;

    let auth = match (host.ssh_key_id.clone(), host.encrypted_password.clone()) {
        (Some(ssh_key_id), Some(password)) => {
            let key = load_owner_key(pool, &username, ssh_key_id).await?;
            ConnectAuthMaterial::PublicKeyAndPassword {
                private_key: key.encrypted_private_key,
                passphrase: key.encrypted_passphrase,
                password,
            }
        }
        (Some(ssh_key_id), None) => {
            let key = load_owner_key(pool, &username, ssh_key_id).await?;
            ConnectAuthMaterial::PublicKey {
                private_key: key.encrypted_private_key,
                passphrase: key.encrypted_passphrase,
            }
        }
        (None, Some(password)) => ConnectAuthMaterial::Password { password },
        (None, None) => {
            return Err(ServiceError::Invalid(
                "host has no connectable authentication material".to_string(),
            ));
        }
    };

    Ok(ConnectMaterial {
        issuance_id: Uuid::new_v4().to_string(),
        host_id: host.id,
        host: host.host,
        port: host.port,
        username: host.username,
        issued_for_username: username,
        issued_for_session_id: session_id,
        expires_at: Utc::now() + Duration::minutes(CONNECT_SECRET_TTL_MINUTES),
        auth,
    })
}

async fn load_owner_key(
    pool: crate::db::DbPool,
    username: &str,
    ssh_key_id: String,
) -> Result<orm::models::SshKey, ServiceError> {
    key_repository::get(pool, username.to_string(), ssh_key_id)
        .await
        .map_err(map_key_error)?
        .ok_or_else(|| ServiceError::NotFound("ssh key not found".to_string()))
}

fn map_host_error(error: host_repository::RepositoryError) -> ServiceError {
    match error {
        host_repository::RepositoryError::NotFound(message) => ServiceError::NotFound(message),
        host_repository::RepositoryError::Internal(message) => ServiceError::Internal(message),
    }
}

fn map_key_error(error: key_repository::RepositoryError) -> ServiceError {
    match error {
        key_repository::RepositoryError::NotFound(message) => ServiceError::NotFound(message),
        key_repository::RepositoryError::Internal(message) => ServiceError::Internal(message),
    }
}
