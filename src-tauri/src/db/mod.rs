pub mod models;
pub mod repository;

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, sqlite::SqliteConnectOptions};
use sqlx::migrate::MigrateError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sql(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migrate(#[from] MigrateError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type DbResult<T> = Result<T, DbError>;

/// Get the app data directory for the database
pub fn db_path() -> DbResult<PathBuf> {
    let mut path = dirs::data_dir()
        .ok_or_else(|| DbError::NotFound("Could not find data directory".to_string()))?;

    path.push("noverterm");
    std::fs::create_dir_all(&path)?;

    path.push("noverterm.db");
    Ok(path)
}

/// Initialize the database and run migrations
pub async fn init_db() -> DbResult<SqlitePool> {
    let path = db_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use SqliteConnectOptions for better path handling
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// Database state for Tauri
pub type DbPool = SqlitePool;
