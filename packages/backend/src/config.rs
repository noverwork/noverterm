use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub auth_secret: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            host: env_value("APP_HOST").unwrap_or_else(|| "127.0.0.1".to_string()),
            port: env_value("APP_PORT")
                .unwrap_or_else(|| "3000".to_string())
                .parse()
                .map_err(|e| format!("invalid APP_PORT: {e}"))?,
            database_url: required_env_value("DATABASE_URL")?,
            auth_secret: env_value("NOVERTERM_AUTH_SECRET")
                .unwrap_or_else(|| "development-only-noverterm-auth-secret".to_string()),
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub fn env_candidates() -> [PathBuf; 2] {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [manifest_dir.join(".env"), manifest_dir.join("../migrator/.env")]
}

pub fn env_value(key: &str) -> Option<String> {
    env::var(key).ok().or_else(|| env_file_value(key))
}

fn required_env_value(key: &str) -> Result<String, String> {
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

#[cfg(test)]
mod tests {
    use super::{env_candidates, env_value, AppConfig};

    #[test]
    fn runtime_reader_reads_database_url_from_repo_env_files() {
        let loaded_database_url = env_value("DATABASE_URL").expect("DATABASE_URL should load");
        let expected_database_url = env_candidates()
            .into_iter()
            .find_map(|path| {
                std::fs::read_to_string(path).ok().and_then(|contents| {
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
    fn config_bind_address_matches_repo_env_defaults() {
        let config = AppConfig::from_env().expect("config should load");
        assert_eq!(config.bind_address(), "127.0.0.1:3000");
    }
}
