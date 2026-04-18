use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
use std::env;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool() -> DbPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment");

    let manager = ConnectionManager::<PgConnection>::new(&database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("failed to create postgres connection pool")
}

pub async fn run_db<T, E, F>(pool: DbPool, operation: F) -> Result<T, E>
where
    T: Send + 'static,
    E: From<String> + Send + 'static,
    F: FnOnce(&mut PgConnection) -> Result<T, E> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut connection = pool
            .get()
            .map_err(|error| E::from(format!("failed to acquire db connection: {error}")))?;
        operation(&mut connection)
    })
    .await
    .map_err(|error| E::from(format!("database task join error: {error}")))?
}
