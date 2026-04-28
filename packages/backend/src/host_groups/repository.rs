use chrono::Utc;
use diesel::prelude::*;
use diesel::OptionalExtension;
use orm::models::{HostGroup, NewHostGroup, UpdateHostGroup};
use orm::schema::host_groups;
use uuid::Uuid;

use crate::bootstrap::db::run_db;
use crate::db::DbPool;

#[derive(Debug)]
pub enum RepositoryError {
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl From<String> for RepositoryError {
    fn from(value: String) -> Self {
        Self::Internal(value)
    }
}

#[derive(Debug, Clone)]
pub struct CreateHostGroupInput {
    pub owner_id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct UpdateHostGroupInput {
    pub owner_id: String,
    pub id: String,
    pub name: String,
}

pub async fn list(pool: DbPool, owner_id: String) -> Result<Vec<HostGroup>, RepositoryError> {
    run_db(pool, move |connection| {
        host_groups::table
            .filter(host_groups::owner_id.eq(owner_id))
            .order(host_groups::name.asc())
            .load::<HostGroup>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn create(
    pool: DbPool,
    input: CreateHostGroupInput,
) -> Result<HostGroup, RepositoryError> {
    run_db(pool, move |connection| {
        let name = normalize_name(input.name)?;
        let now = Utc::now().naive_utc();
        let new_group = NewHostGroup {
            id: Uuid::new_v4().to_string(),
            owner_id: input.owner_id,
            name,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(host_groups::table)
            .values(&new_group)
            .get_result::<HostGroup>(connection)
            .map_err(map_insert_error)
    })
    .await
}

pub async fn update(
    pool: DbPool,
    input: UpdateHostGroupInput,
) -> Result<HostGroup, RepositoryError> {
    run_db(pool, move |connection| {
        let changes = UpdateHostGroup {
            name: normalize_name(input.name)?,
            updated_at: Utc::now().naive_utc(),
        };

        diesel::update(
            host_groups::table
                .filter(host_groups::owner_id.eq(input.owner_id))
                .filter(host_groups::id.eq(input.id)),
        )
        .set(changes)
        .get_result::<HostGroup>(connection)
        .optional()
        .map_err(internal_error)?
        .ok_or_else(|| RepositoryError::NotFound("host group not found".to_string()))
    })
    .await
}

pub async fn delete(pool: DbPool, owner_id: String, id: String) -> Result<bool, RepositoryError> {
    run_db(pool, move |connection| {
        diesel::delete(
            host_groups::table
                .filter(host_groups::owner_id.eq(owner_id))
                .filter(host_groups::id.eq(id)),
        )
        .execute(connection)
        .map(|rows| rows > 0)
        .map_err(internal_error)
    })
    .await
}

fn normalize_name(name: String) -> Result<String, RepositoryError> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        Err(RepositoryError::Conflict(
            "host group name is required".to_string(),
        ))
    } else {
        Ok(trimmed)
    }
}

fn map_insert_error(error: diesel::result::Error) -> RepositoryError {
    match error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => RepositoryError::Conflict("host group already exists".to_string()),
        other => internal_error(other),
    }
}

fn internal_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Internal(format!("host group repository error: {error}"))
}
