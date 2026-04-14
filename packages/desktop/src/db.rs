use diesel::r2d2::{ConnectionManager, Pool as R2d2Pool};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub type Pool = R2d2Pool<ConnectionManager<SqliteConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn init_pool(db_path: &str) -> Pool {
    let manager = ConnectionManager::<SqliteConnection>::new(db_path);

    R2d2Pool::builder()
        .build(manager)
        .expect("failed to create sqlite connection pool")
}

pub fn run_migrations(pool: &Pool) {
    let mut connection = pool
        .get()
        .expect("failed to get sqlite connection for migrations");

    connection
        .run_pending_migrations(MIGRATIONS)
        .expect("failed to run database migrations");
}
