use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager};
pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool() -> Result<DbPool, String> {
    let database_url = crate::bootstrap::required_env_value("DATABASE_URL")?;

    let manager = ConnectionManager::<PgConnection>::new(&database_url);
    r2d2::Pool::builder()
        .build(manager)
        .map_err(|error| format!("failed to create postgres connection pool: {error}"))
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
