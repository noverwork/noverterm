mod db;

use axum::{routing::get, Router};
use tracing_subscriber::EnvFilter;

async fn healthcheck() -> &'static str {
    "Backend running"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let pool = db::init_pool();
    tracing::info!("Database pool initialized");

    let app = Router::new()
        .route("/", get(healthcheck))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind backend listener");

    tracing::info!("Backend running on 127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("backend server error");
}
