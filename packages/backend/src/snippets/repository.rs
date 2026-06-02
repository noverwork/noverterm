use chrono::Utc;
use diesel::prelude::*;
use diesel::OptionalExtension;
use orm::models::{HostSnippet, NewHostSnippet, UpdateHostSnippet};
use orm::schema::{host_snippets, ssh_hosts};
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
pub struct CreateSnippetInput {
    pub owner_id: String,
    pub host_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct UpdateSnippetInput {
    pub owner_id: String,
    pub id: String,
    pub host_id: String,
    pub title: String,
    pub body: String,
}

pub async fn list_all(pool: DbPool, owner_id: String) -> Result<Vec<HostSnippet>, RepositoryError> {
    run_db(pool, move |connection| {
        host_snippets::table
            .filter(host_snippets::owner_id.eq(owner_id))
            .order(host_snippets::created_at.desc())
            .load::<HostSnippet>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn get(
    pool: DbPool,
    owner_id: String,
    id: String,
) -> Result<Option<HostSnippet>, RepositoryError> {
    run_db(pool, move |connection| {
        host_snippets::table
            .filter(host_snippets::owner_id.eq(owner_id))
            .filter(host_snippets::id.eq(id))
            .first::<HostSnippet>(connection)
            .optional()
            .map_err(internal_error)
    })
    .await
}

pub async fn create(
    pool: DbPool,
    input: CreateSnippetInput,
) -> Result<HostSnippet, RepositoryError> {
    run_db(pool, move |connection| {
        ensure_owner_scoped_host_exists(connection, &input.owner_id, &input.host_id)?;

        let now = Utc::now().naive_utc();
        let new_snippet = NewHostSnippet {
            id: Uuid::new_v4().to_string(),
            host_id: input.host_id,
            owner_id: input.owner_id,
            title: input.title,
            body: input.body,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(host_snippets::table)
            .values(&new_snippet)
            .get_result::<HostSnippet>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn update(
    pool: DbPool,
    input: UpdateSnippetInput,
) -> Result<HostSnippet, RepositoryError> {
    run_db(pool, move |connection| {
        ensure_owner_scoped_host_exists(connection, &input.owner_id, &input.host_id)?;

        let changes = UpdateHostSnippet {
            host_id: input.host_id,
            title: input.title,
            body: input.body,
            updated_at: Utc::now().naive_utc(),
        };

        diesel::update(
            host_snippets::table
                .filter(host_snippets::owner_id.eq(input.owner_id))
                .filter(host_snippets::id.eq(input.id)),
        )
        .set(changes)
        .get_result::<HostSnippet>(connection)
        .optional()
        .map_err(internal_error)?
        .ok_or_else(|| RepositoryError::NotFound("snippet not found".to_string()))
    })
    .await
}

pub async fn delete(pool: DbPool, owner_id: String, id: String) -> Result<bool, RepositoryError> {
    run_db(pool, move |connection| {
        diesel::delete(
            host_snippets::table
                .filter(host_snippets::owner_id.eq(owner_id))
                .filter(host_snippets::id.eq(id)),
        )
        .execute(connection)
        .map(|rows| rows > 0)
        .map_err(internal_error)
    })
    .await
}

fn ensure_owner_scoped_host_exists(
    connection: &mut PgConnection,
    owner_id: &str,
    host_id: &str,
) -> Result<(), RepositoryError> {
    let host_exists = ssh_hosts::table
        .filter(ssh_hosts::owner_id.eq(owner_id))
        .filter(ssh_hosts::id.eq(host_id))
        .select(ssh_hosts::id)
        .first::<String>(connection)
        .optional()
        .map_err(internal_error)?
        .is_some();

    if host_exists {
        Ok(())
    } else {
        Err(RepositoryError::NotFound("ssh host not found".to_string()))
    }
}

pub async fn get_host_name(
    pool: DbPool,
    host_id: String,
) -> Result<Option<String>, RepositoryError> {
    run_db(pool, move |connection| {
        ssh_hosts::table
            .filter(ssh_hosts::id.eq(host_id))
            .select(ssh_hosts::name)
            .first::<String>(connection)
            .optional()
            .map_err(internal_error)
    })
    .await
}

fn internal_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Internal(format!("snippet repository error: {error}"))
}
