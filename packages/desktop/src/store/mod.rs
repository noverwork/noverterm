pub mod groups;
pub mod hosts;
pub mod keys;
pub mod links;
pub mod port_forwards;
pub mod settings;
pub mod snippets;
pub mod ssh_config;

use std::path::{Path, PathBuf};

use diesel::connection::SimpleConnection;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

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
pub async fn db_backup(pool: crate::app::State<'_, DbPool>) -> Result<String, String> {
    let download_dir = dirs::download_dir().ok_or("no download folder available")?;
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

/// The database file in the app's data folder. Debug builds get their own so
/// `cargo make dev` cannot scribble over the installed app's connections and
/// host keys.
pub fn database_path() -> Result<PathBuf, String> {
    let name = if cfg!(debug_assertions) {
        "noverterm-dev.db"
    } else {
        "noverterm.db"
    };
    Ok(crate::app::data_dir()?.join(name))
}

/// SQLite keeps a database in up to three files.
const SQLITE_SUFFIXES: [&str; 3] = ["", "-wal", "-shm"];

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

/// Moves a database with its WAL and shared-memory files, dropping stale ones
/// at the destination.
fn move_database(from: &Path, to: &Path) -> std::io::Result<()> {
    for suffix in SQLITE_SUFFIXES {
        let (source, target) = (with_suffix(from, suffix), with_suffix(to, suffix));
        if source.exists() {
            std::fs::rename(source, target)?;
        } else if target.exists() {
            std::fs::remove_file(target)?;
        }
    }
    Ok(())
}

/// Fails unless `source` is a Noverterm database. Opening it does not change it.
pub fn check_backup(source: &Path) -> Result<(), String> {
    use diesel::Connection as _;
    let mut connection = SqliteConnection::establish(&source.to_string_lossy())
        .map_err(|error| format!("cannot open backup: {error}"))?;
    connection
        .batch_execute("SELECT 1 FROM __diesel_schema_migrations LIMIT 1")
        .map_err(|_| "not a Noverterm backup".to_string())
}

/// Copies `source` next to `database`, where the next launch swaps it in; the
/// open database cannot be replaced while the app uses it. Opening the copy
/// runs the migrations, which brings an older backup up to date.
pub fn stage_restore(source: &Path, database: &Path) -> Result<(), String> {
    check_backup(source)?;
    let staged = with_suffix(database, ".restore");
    let staged_ok = std::fs::copy(source, &staged)
        .map_err(|error| format!("cannot copy backup: {error}"))
        .and_then(|_| init_pool(&staged).map(drop));
    if staged_ok.is_err() {
        for suffix in SQLITE_SUFFIXES {
            let _ = std::fs::remove_file(with_suffix(&staged, suffix));
        }
    }
    staged_ok
}

/// [`stage_restore`] for the app's own database.
pub async fn db_restore(source: PathBuf) -> Result<(), String> {
    let database = database_path()?;
    tokio::task::spawn_blocking(move || stage_restore(&source, &database))
        .await
        .map_err(|error| format!("restore task join error: {error}"))?
}

/// Swaps in a backup staged by [`stage_restore`]. Runs before the database is
/// opened; the replaced database stays beside it as `.before-restore`.
pub fn apply_pending_restore(database: &Path) -> Result<(), String> {
    let staged = with_suffix(database, ".restore");
    if !staged.exists() {
        return Ok(());
    }
    move_database(database, &with_suffix(database, ".before-restore"))
        .and_then(|()| move_database(&staged, database))
        .map_err(|error| format!("failed to restore backup: {error}"))
}

#[cfg(test)]
pub(crate) fn test_pool(directory: &tempfile::TempDir) -> DbPool {
    init_pool(&directory.path().join("noverterm.db")).expect("open test database")
}

#[cfg(test)]
mod tests;
