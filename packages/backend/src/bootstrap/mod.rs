pub mod app;
pub mod db;
mod state;

use std::env;
use std::fs;
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;

#[cfg(test)]
pub use app::build_test_router;
#[cfg(test)]
pub use app::build_test_router_with_db;
#[cfg(test)]
pub use state::test_app_state;
#[cfg(test)]
pub use state::test_app_state_with_db;
pub use state::AppState;

pub async fn run() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let pool = crate::db::init_pool()?;
    tracing::info!("Database pool initialized");

    let app_state = AppState::from_env(Some(pool));
    let app = app::build_router(app_state);
    let bind_address = backend_bind_address()?;

    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AddrInUse {
                format!(
                    "backend address {bind_address} is already in use; stop the existing process or change BACKEND_HOST/BACKEND_PORT in packages/backend/.env"
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

pub(crate) fn env_candidates() -> [PathBuf; 2] {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [manifest_dir.join(".env"), manifest_dir.join("../migrator/.env")]
}

pub(crate) fn env_value(key: &str) -> Option<String> {
    env::var(key).ok().or_else(|| env_file_value(key))
}

pub(crate) fn required_env_value(key: &str) -> Result<String, String> {
    env_value(key).ok_or_else(|| format!("{key} must be set in environment or backend env file"))
}

fn env_file_value(key: &str) -> Option<String> {
    env_candidates().into_iter().find_map(|path| {
        fs::read_to_string(path).ok().and_then(|contents| {
            contents.lines().find_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return None;
                }

                let (candidate_key, value) = trimmed.split_once('=')?;
                (candidate_key.trim() == key).then(|| value.trim().to_string())
            })
        })
    })
}

fn backend_bind_address() -> Result<String, String> {
    let host = env_value("BACKEND_HOST").unwrap_or_else(|| "127.0.0.1".to_string());
    let port = env_value("BACKEND_PORT")
        .unwrap_or_else(|| "3000".to_string())
        .parse::<u16>()
        .map_err(|error| format!("invalid BACKEND_PORT: {error}"))?;

    Ok(format!("{host}:{port}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{backend_bind_address, env_candidates, env_value};

    #[test]
    fn runtime_env_reader_reads_database_url_from_repo_env_files() {
        let loaded_database_url = env_value("DATABASE_URL").expect("DATABASE_URL should load");
        let expected_database_url = env_candidates()
            .into_iter()
            .find_map(|path| {
                fs::read_to_string(path).ok().and_then(|contents| {
                    contents
                        .lines()
                        .find_map(|line| line.strip_prefix("DATABASE_URL="))
                        .map(str::to_string)
                })
            })
            .expect("repo should provide a DATABASE_URL in env files");

        assert_eq!(loaded_database_url, expected_database_url);
    }

    #[test]
    fn backend_bind_address_matches_repo_env_defaults() {
        let bind_address = backend_bind_address().expect("bind address should parse");
        assert_eq!(bind_address, "127.0.0.1:3000");
    }
}
