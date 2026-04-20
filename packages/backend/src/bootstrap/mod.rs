pub mod app;
pub mod db;
mod state;

use tracing_subscriber::EnvFilter;

use crate::config::AppConfig;

#[cfg(test)]
pub use app::build_test_router;
#[cfg(test)]
pub use state::test_app_state;
pub use state::AppState;

pub async fn run() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env()?;
    let pool = crate::db::init_pool(&config.database_url)?;
    tracing::info!("Database pool initialized");

    let app_state = AppState::new(&config, pool);
    let app = app::build_router(app_state);
    let bind_address = config.bind_address();

    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AddrInUse {
                format!(
                    "backend address {bind_address} is already in use; stop the existing process or change APP_HOST/APP_PORT in packages/backend/.env"
                )
            } else {
                format!("failed to bind backend listener on {bind_address}: {error}")
            }
        })?;

    tracing::info!("Backend running on {bind_address}");

    axum::serve(listener, app)
        .await
        .map_err(|error| format!("backend server error: {error}"))
}
