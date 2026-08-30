pub mod groups;
pub mod hosts;
pub mod keys;
pub mod settings;
pub mod snippets;

use std::path::Path;

use diesel::connection::SimpleConnection;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tauri::Manager;

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// SQLite needs these set per connection: foreign keys are off by default, and
/// WAL plus a busy timeout keep the UI thread from tripping over the writer.
///
/// `busy_timeout` has to come first. Switching a fresh database to WAL takes a
/// brief exclusive lock, and without a timeout already in place every
/// connection racing for it fails outright with "database is locked".
#[derive(Debug)]
struct SqlitePragmas;

impl CustomizeConnection<SqliteConnection, r2d2::Error> for SqlitePragmas {
    fn on_acquire(&self, connection: &mut SqliteConnection) -> Result<(), r2d2::Error> {
        connection
            .batch_execute(
                "PRAGMA busy_timeout = 5000; \
                 PRAGMA foreign_keys = ON; \
                 PRAGMA journal_mode = WAL;",
            )
            .map_err(r2d2::Error::QueryError)
    }
}

pub fn init_pool(database_path: &Path) -> Result<DbPool, String> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_path.to_string_lossy());
    // SQLite serialises writers anyway, so a large pool buys nothing.
    let pool = r2d2::Pool::builder()
        .max_size(4)
        .connection_customizer(Box::new(SqlitePragmas))
        .build(manager)
        .map_err(|error| format!("failed to open database: {error}"))?;

    pool.get()
        .map_err(|error| format!("failed to acquire db connection: {error}"))?
        .run_pending_migrations(MIGRATIONS)
        .map_err(|error| format!("failed to run migrations: {error}"))?;

    Ok(pool)
}

pub async fn run_db<T, F>(pool: DbPool, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&mut SqliteConnection) -> Result<T, String> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut connection = pool
            .get()
            .map_err(|error| format!("failed to acquire db connection: {error}"))?;
        operation(&mut connection)
    })
    .await
    .map_err(|error| format!("database task join error: {error}"))?
}

pub fn internal_error(error: diesel::result::Error) -> String {
    format!("database error: {error}")
}

/// Writes a copy of the database to the user's download folder and returns the
/// file name. `VACUUM INTO` is atomic and folds in the WAL, so unlike copying
/// the file it is safe to run while the app is using the database.
#[tauri::command]
#[specta::specta]
pub async fn db_backup(
    app: tauri::AppHandle,
    pool: tauri::State<'_, DbPool>,
) -> Result<String, String> {
    let download_dir = app
        .path()
        .download_dir()
        .map_err(|error| format!("no download folder available: {error}"))?;
    let file_name = format!(
        "noverterm-backup-{}.db",
        chrono::Local::now().format("%Y-%m-%d-%H-%M-%S")
    );
    let destination = download_dir.join(&file_name);

    if destination.exists() {
        return Err(format!("{file_name} already exists"));
    }

    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let escaped = destination.to_string_lossy().replace('\'', "''");
        connection
            .batch_execute(&format!("VACUUM INTO '{escaped}'"))
            .map_err(|error| format!("backup failed: {error}"))
    })
    .await?;

    Ok(file_name)
}

#[cfg(test)]
pub(crate) fn test_pool(directory: &tempfile::TempDir) -> DbPool {
    init_pool(&directory.path().join("noverterm.db")).expect("open test database")
}

#[cfg(test)]
mod tests;
