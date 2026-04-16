use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Initialize a sqlite connection pool.
pub fn init_pool(db_path: &str) -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);

    r2d2::Pool::builder()
        .build(manager)
        .expect("failed to create sqlite connection pool")
}

/// Run pending migrations against the pool.
pub fn run_migrations(pool: &DbPool) {
    let mut conn = pool
        .get()
        .expect("failed to get db connection for migrations");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("failed to run database migrations");
}
