use std::env;
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::email::{PasswordResetEmailConfig, PasswordResetSmtpTlsMode};

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub backend_url: String,
    pub password_reset_email: PasswordResetEmailConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            host: required_string_env("APP_HOST")?,
            port: required_number_env("APP_PORT")?,
            database_url: required_string_env("DATABASE_URL")?,
            jwt_secret: required_string_env("JWT_SECRET")?,
            backend_url: required_string_env("BACKEND_URL")?,
            password_reset_email: password_reset_email_config()?,
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn password_reset_url(&self) -> String {
        let base_url = self
            .backend_url
            .trim_end_matches('/')
            .strip_suffix("/api")
            .unwrap_or_else(|| self.backend_url.trim_end_matches('/'));
        format!("{base_url}/reset-password")
    }
}

pub fn env_candidates() -> [PathBuf; 2] {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        manifest_dir.join(".env"),
        manifest_dir.join("../migrator/.env"),
    ]
}

pub fn env_value(key: &str) -> Option<String> {
    env::var(key).ok().or_else(|| env_file_value(key))
}

pub fn required_string_env(key: &str) -> Result<String, String> {
    env_value(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{key} must be set in environment or backend env file"))
}

pub fn required_number_env<T>(key: &str) -> Result<T, String>
where
    T: FromStr,
    T::Err: Display,
{
    required_string_env(key)?
        .parse()
        .map_err(|error| format!("invalid {key}: {error}"))
}

pub fn required_boolean_env(key: &str) -> Result<bool, String> {
    match required_string_env(key)?.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid {key}: expected boolean true or false")),
    }
}

fn password_reset_email_config() -> Result<PasswordResetEmailConfig, String> {
    let smtp_port: u16 = required_number_env("SMTP_PORT")?;
    let tls_mode =
        PasswordResetSmtpTlsMode::from_env_value(&required_string_env("SMTP_TLS_MODE")?)?;

    Ok(PasswordResetEmailConfig {
        smtp_host: required_string_env("SMTP_HOST")?,
        smtp_port,
        tls_mode,
        smtp_username: required_string_env("SMTP_USERNAME")?,
        smtp_password: required_string_env("SMTP_PASSWORD")?,
        from: required_string_env("SMTP_FROM")?,
    })
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
    use crate::email::{PasswordResetEmailConfig, PasswordResetSmtpTlsMode};

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
        let config = AppConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            database_url: "postgres://postgres:postgres@localhost:5432/app".to_string(),
            jwt_secret: "backend-test-secret".to_string(),
            backend_url: "https://noverterm.noverwork.com".to_string(),
            password_reset_email: PasswordResetEmailConfig {
                smtp_host: "smtp.example.com".to_string(),
                smtp_port: 587,
                tls_mode: PasswordResetSmtpTlsMode::StartTls,
                smtp_username: "user".to_string(),
                smtp_password: "password".to_string(),
                from: "Noverterm <noreply@example.com>".to_string(),
            },
        };
        assert_eq!(config.bind_address(), "127.0.0.1:3000");
        assert_eq!(
            config.password_reset_url(),
            "https://noverterm.noverwork.com/reset-password"
        );
    }
}
