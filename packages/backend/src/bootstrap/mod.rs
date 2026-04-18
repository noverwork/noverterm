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

pub async fn run() {
    load_runtime_env();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let pool = crate::db::init_pool();
    tracing::info!("Database pool initialized");

    let app_state = AppState::from_env(Some(pool));
    let app = app::build_router(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind backend listener");

    tracing::info!("Backend running on 127.0.0.1:3000");

    axum::serve(listener, app)
        .await
        .expect("backend server error");
}

fn load_runtime_env() {
    for path in env_candidates() {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };

            if env::var_os(key.trim()).is_none() {
                env::set_var(key.trim(), value.trim());
            }
        }
    }
}

fn env_candidates() -> [PathBuf; 2] {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [manifest_dir.join(".env"), manifest_dir.join("../migrator/.env")]
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    use super::{env_candidates, load_runtime_env};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn runtime_env_loader_reads_database_url_from_repo_env_files() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should be acquirable");

        let original_database_url = env::var_os("DATABASE_URL");
        env::remove_var("DATABASE_URL");

        load_runtime_env();

        let loaded_database_url = env::var("DATABASE_URL").expect("DATABASE_URL should load");
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

        match original_database_url {
            Some(value) => env::set_var("DATABASE_URL", value),
            None => env::remove_var("DATABASE_URL"),
        }
    }
}
