use chrono::Utc;
use diesel::prelude::*;
use diesel::OptionalExtension;
use orm::models::{NewUserSetting, UpdateUserSetting, UserSetting};
use orm::schema::user_settings;
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

pub async fn list(pool: DbPool, owner_id: String) -> Result<Vec<UserSetting>, RepositoryError> {
    run_db(pool, move |connection| {
        user_settings::table
            .filter(user_settings::owner_id.eq(owner_id))
            .order(user_settings::key.asc())
            .load::<UserSetting>(connection)
            .map_err(internal_error)
    })
    .await
}

pub async fn get(
    pool: DbPool,
    owner_id: String,
    key: String,
) -> Result<Option<UserSetting>, RepositoryError> {
    run_db(pool, move |connection| {
        user_settings::table
            .filter(user_settings::owner_id.eq(owner_id))
            .filter(user_settings::key.eq(key))
            .first::<UserSetting>(connection)
            .optional()
            .map_err(internal_error)
    })
    .await
}

pub async fn create(
    pool: DbPool,
    owner_id: String,
    key: String,
    value: String,
) -> Result<UserSetting, RepositoryError> {
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        let new_setting = NewUserSetting {
            id: Uuid::new_v4().to_string(),
            owner_id,
            key,
            value,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(user_settings::table)
            .values(&new_setting)
            .get_result::<UserSetting>(connection)
            .map_err(|error| match error {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => RepositoryError::Conflict("setting already exists".to_string()),
                _ => internal_error(error),
            })
    })
    .await
}

pub async fn update(
    pool: DbPool,
    owner_id: String,
    key: String,
    value: String,
) -> Result<UserSetting, RepositoryError> {
    run_db(pool, move |connection| {
        let changes = UpdateUserSetting {
            value,
            updated_at: Utc::now().naive_utc(),
        };

        diesel::update(
            user_settings::table
                .filter(user_settings::owner_id.eq(owner_id))
                .filter(user_settings::key.eq(key)),
        )
        .set(changes)
        .get_result::<UserSetting>(connection)
        .optional()
        .map_err(internal_error)?
        .ok_or_else(|| RepositoryError::NotFound("setting not found".to_string()))
    })
    .await
}

pub async fn delete(pool: DbPool, owner_id: String, key: String) -> Result<bool, RepositoryError> {
    run_db(pool, move |connection| {
        diesel::delete(
            user_settings::table
                .filter(user_settings::owner_id.eq(owner_id))
                .filter(user_settings::key.eq(key)),
        )
        .execute(connection)
        .map(|rows| rows > 0)
        .map_err(internal_error)
    })
    .await
}

fn internal_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::Internal(format!("settings repository error: {error}"))
}
