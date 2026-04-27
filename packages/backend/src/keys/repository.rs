use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewSshKey, SshKey, UpdateSshKey};
use orm::schema::ssh_keys;
use uuid::Uuid;

use crate::bootstrap::db::run_db;
use crate::db::DbPool;

#[derive(Debug)]
pub enum RepositoryError {
    NotFound(String),
    Internal(String),
}

impl From<String> for RepositoryError {
    fn from(value: String) -> Self {
        Self::Internal(value)
    }
}

#[derive(Debug, Clone)]
pub struct CreateKeyInput {
    pub owner_id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateKeyInput {
    pub owner_id: String,
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub encrypted_passphrase: Option<String>,
}

pub async fn list(pool: DbPool, owner_id: String) -> Result<Vec<SshKey>, RepositoryError> {
    run_db(pool, move |connection| {
        ssh_keys::table
            .filter(ssh_keys::owner_id.eq(owner_id))
            .order(ssh_keys::created_at.asc())
            .load::<SshKey>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn get(
    pool: DbPool,
    owner_id: String,
    id: String,
) -> Result<Option<SshKey>, RepositoryError> {
    run_db(pool, move |connection| {
        ssh_keys::table
            .filter(ssh_keys::owner_id.eq(owner_id))
            .filter(ssh_keys::id.eq(id))
            .first::<SshKey>(connection)
            .optional()
            .map_err(internal_error)
    })
    .await
}

pub async fn create(pool: DbPool, input: CreateKeyInput) -> Result<SshKey, RepositoryError> {
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        let new_key = NewSshKey {
            id: Uuid::new_v4().to_string(),
            owner_id: input.owner_id,
            name: input.name,
            kind: input.kind,
            fingerprint: input.fingerprint,
            encrypted_private_key: input.encrypted_private_key,
            encrypted_passphrase: input.encrypted_passphrase,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(ssh_keys::table)
            .values(&new_key)
            .get_result::<SshKey>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn update(pool: DbPool, input: UpdateKeyInput) -> Result<SshKey, RepositoryError> {
    run_db(pool, move |connection| {
        let existing_key = ssh_keys::table
            .filter(ssh_keys::owner_id.eq(&input.owner_id))
            .filter(ssh_keys::id.eq(&input.id))
            .first::<SshKey>(connection)
            .optional()
            .map_err(internal_error)?
            .ok_or_else(|| RepositoryError::NotFound("ssh key not found".to_string()))?;

        let replacing_private_key = input.encrypted_private_key.is_some();
        let changes = UpdateSshKey {
            name: input.name,
            kind: input.kind,
            fingerprint: input.fingerprint.or(existing_key.fingerprint),
            encrypted_private_key: input
                .encrypted_private_key
                .unwrap_or(existing_key.encrypted_private_key),
            encrypted_passphrase: if replacing_private_key {
                input.encrypted_passphrase
            } else {
                existing_key.encrypted_passphrase
            },
            updated_at: Utc::now().naive_utc(),
        };

        diesel::update(
            ssh_keys::table
                .filter(ssh_keys::owner_id.eq(input.owner_id))
                .filter(ssh_keys::id.eq(input.id)),
        )
        .set(changes)
        .get_result::<SshKey>(connection)
        .optional()
        .map_err(internal_error)?
        .ok_or_else(|| RepositoryError::NotFound("ssh key not found".to_string()))
    })
    .await
}

pub async fn delete(pool: DbPool, owner_id: String, id: String) -> Result<bool, RepositoryError> {
    run_db(pool, move |connection| {
        diesel::delete(
            ssh_keys::table
                .filter(ssh_keys::owner_id.eq(owner_id))
                .filter(ssh_keys::id.eq(id)),
        )
        .execute(connection)
        .map(|rows| rows > 0)
        .map_err(internal_error)
    })
    .await
}

fn internal_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Internal(format!("key repository error: {error}"))
}
