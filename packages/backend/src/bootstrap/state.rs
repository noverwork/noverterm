use crate::auth::{AuthConfig, AuthService};
use crate::config::AppConfig;
use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub db_pool: DbPool,
}

impl AppState {
    pub fn new(config: &AppConfig, db_pool: DbPool) -> Self {
        Self {
            auth_service: AuthService::new(
                AuthConfig::new(config.auth_secret.clone()),
                db_pool.clone(),
            ),
            db_pool,
        }
    }

    pub fn require_db_pool(&self) -> Result<DbPool, String> {
        Ok(self.db_pool.clone())
    }
}

#[cfg(test)]
pub fn test_app_state(auth_service: AuthService, db_pool: DbPool) -> AppState {
    AppState {
        auth_service,
        db_pool,
    }
}
