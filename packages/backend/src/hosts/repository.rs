use chrono::Utc;
use diesel::prelude::*;
use diesel::OptionalExtension;
use orm::models::{NewSshHost, SshHost, UpdateSshHost};
use orm::schema::{ssh_hosts, ssh_keys};
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
pub struct CreateHostInput {
    pub owner_id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateHostInput {
    pub owner_id: String,
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
}

pub async fn list(pool: DbPool, owner_id: String) -> Result<Vec<SshHost>, RepositoryError> {
    run_db(pool, move |connection| {
        ssh_hosts::table
            .filter(ssh_hosts::owner_id.eq(owner_id))
            .order(ssh_hosts::created_at.asc())
            .load::<SshHost>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn get(
    pool: DbPool,
    owner_id: String,
    id: String,
) -> Result<Option<SshHost>, RepositoryError> {
    run_db(pool, move |connection| {
        ssh_hosts::table
            .filter(ssh_hosts::owner_id.eq(owner_id))
            .filter(ssh_hosts::id.eq(id))
            .first::<SshHost>(connection)
            .optional()
            .map_err(internal_error)
    })
    .await
}

pub async fn create(pool: DbPool, input: CreateHostInput) -> Result<SshHost, RepositoryError> {
    run_db(pool, move |connection| {
        ensure_owner_scoped_key_exists(connection, &input.owner_id, input.ssh_key_id.as_deref())?;

        let now = Utc::now().naive_utc();
        let new_host = NewSshHost {
            id: Uuid::new_v4().to_string(),
            owner_id: input.owner_id,
            name: input.name,
            host: input.host,
            port: input.port,
            username: input.username,
            auth_mode: input.auth_mode,
            ssh_key_id: input.ssh_key_id,
            encrypted_password: input.encrypted_password,
            created_at: now,
            updated_at: now,
            last_connected_at: None,
        };

        diesel::insert_into(ssh_hosts::table)
            .values(&new_host)
            .get_result::<SshHost>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn update(pool: DbPool, input: UpdateHostInput) -> Result<SshHost, RepositoryError> {
    run_db(pool, move |connection| {
        ensure_owner_scoped_key_exists(connection, &input.owner_id, input.ssh_key_id.as_deref())?;

        let changes = UpdateSshHost {
            name: input.name,
            host: input.host,
            port: input.port,
            username: input.username,
            auth_mode: input.auth_mode,
            ssh_key_id: input.ssh_key_id,
            encrypted_password: input.encrypted_password,
            updated_at: Utc::now().naive_utc(),
            last_connected_at: None,
        };

        diesel::update(
            ssh_hosts::table
                .filter(ssh_hosts::owner_id.eq(input.owner_id))
                .filter(ssh_hosts::id.eq(input.id)),
        )
        .set(changes)
        .get_result::<SshHost>(connection)
        .optional()
        .map_err(internal_error)?
        .ok_or_else(|| RepositoryError::NotFound("host not found".to_string()))
    })
    .await
}

pub async fn delete(pool: DbPool, owner_id: String, id: String) -> Result<bool, RepositoryError> {
    run_db(pool, move |connection| {
        diesel::delete(
            ssh_hosts::table
                .filter(ssh_hosts::owner_id.eq(owner_id))
                .filter(ssh_hosts::id.eq(id)),
        )
        .execute(connection)
        .map(|rows| rows > 0)
        .map_err(internal_error)
    })
    .await
}

fn ensure_owner_scoped_key_exists(
    connection: &mut PgConnection,
    owner_id: &str,
    ssh_key_id: Option<&str>,
) -> Result<(), RepositoryError> {
    let Some(ssh_key_id) = ssh_key_id else {
        return Ok(());
    };

    let key_exists = ssh_keys::table
        .filter(ssh_keys::owner_id.eq(owner_id))
        .filter(ssh_keys::id.eq(ssh_key_id))
        .select(ssh_keys::id)
        .first::<String>(connection)
        .optional()
        .map_err(internal_error)?
        .is_some();

    if key_exists {
        Ok(())
    } else {
        Err(RepositoryError::NotFound("ssh key not found".to_string()))
    }
}

fn internal_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Internal(format!("host repository error: {error}"))
}
