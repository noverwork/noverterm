use crate::auth::{AuthConfig, AuthService};
use crate::db::DbPool;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub db_pool: Option<DbPool>,
}

impl AppState {
    pub fn new(auth_service: AuthService, db_pool: Option<DbPool>) -> Self {
        Self {
            auth_service,
            db_pool,
        }
    }

    pub fn from_env(db_pool: Option<DbPool>) -> Self {
        Self::new(AuthService::new(AuthConfig::from_env()), db_pool)
    }

    pub fn require_db_pool(&self) -> Result<DbPool, String> {
        self.db_pool
            .clone()
            .ok_or_else(|| "database unavailable".to_string())
    }
}

#[cfg(test)]
pub fn test_app_state(auth_service: AuthService) -> AppState {
    AppState::new(auth_service, None)
}

#[cfg(test)]
pub fn test_app_state_with_db(auth_service: AuthService, db_pool: DbPool) -> AppState {
    AppState::new(auth_service, Some(db_pool))
}
