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

    let app = Router::new().route("/", get(healthcheck));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind backend listener");

    println!("Backend running");

    axum::serve(listener, app)
        .await
        .expect("backend server error");
}
