use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use orm::models::{NewUser, User};
use orm::schema::users;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::token::{hash_password, hash_token, TokenService};
use crate::db::{run_db, DbPool};

#[derive(Clone)]
pub struct AuthConfig {
    token_service: TokenService,
}

#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
    pool: DbPool,
    sessions: Arc<RwLock<SessionState>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub username: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: chrono::DateTime<Utc>,
    pub username: String,
}

#[derive(Debug, Default)]
struct SessionState {
    active_refresh_tokens: HashMap<String, String>,
    retired_refresh_tokens: HashMap<String, String>,
    sessions: HashMap<String, SessionRecord>,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    username: String,
    refresh_token_hash: String,
    refresh_expires_at: chrono::DateTime<Utc>,
    revoked: bool,
}

impl AuthConfig {
    pub fn new(secret: String) -> Self {
        Self {
            token_service: TokenService::new(secret, Duration::minutes(15), Duration::days(30)),
        }
    }
}

impl AuthService {
    pub fn new(config: AuthConfig, pool: DbPool) -> Self {
        Self {
            config,
            pool,
            sessions: Arc::new(RwLock::new(SessionState::default())),
        }
    }

    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, String> {
        let username = request.username.clone();
        let check_username = username.clone();
        let existing: Result<Option<User>, String> = run_db(self.pool.clone(), move |conn| {
            users::table
                .filter(users::username.eq(&check_username))
                .first::<User>(conn)
                .optional()
                .map_err(|e| format!("database error: {e}"))
        })
        .await;

        if existing?.is_some() {
            return Err("username already exists".to_string());
        }

        let password_hash = hash_password(&request.password);
        let now = Utc::now().naive_utc();
        let new_user = NewUser {
            id: Uuid::new_v4().to_string(),
            username: request.username.clone(),
            password_hash,
            created_at: now,
            updated_at: now,
        };

        run_db(self.pool.clone(), move |conn| {
            diesel::insert_into(users::table)
                .values(&new_user)
                .execute(conn)
                .map_err(|e| format!("database error: {e}"))
        })
        .await?;

        self.create_session(&username).await
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, String> {
        let username = request.username.clone();
        let check_username = username.clone();
        let user: Option<User> = run_db(self.pool.clone(), move |conn| {
            users::table
                .filter(users::username.eq(&check_username))
                .first::<User>(conn)
                .optional()
                .map_err(|e| format!("database error: {e}"))
        })
        .await?;

        let Some(user) = user else {
            return Err("invalid credentials".to_string());
        };

        if hash_password(&request.password) != user.password_hash {
            return Err("invalid credentials".to_string());
        }

        self.create_session(&username).await
    }

    pub async fn refresh(&self, request: RefreshRequest) -> Result<AuthResponse, String> {
        let refresh_token_hash = hash_token(&request.refresh_token);
        let mut sessions = self.sessions.write().await;

        if let Some(session_id) = sessions
            .retired_refresh_tokens
            .get(&refresh_token_hash)
            .cloned()
        {
            self.revoke_session_locked(&mut sessions, &session_id);
            return Err("refresh token reuse detected".to_string());
        }

        let Some(session_id) = sessions
            .active_refresh_tokens
            .get(&refresh_token_hash)
            .cloned()
        else {
            return Err("invalid refresh token".to_string());
        };

        let Some(session) = sessions.sessions.get(&session_id) else {
            sessions.active_refresh_tokens.remove(&refresh_token_hash);
            return Err("invalid refresh token".to_string());
        };

        let username = session.username.clone();
        let session_revoked = session.revoked;
        let refresh_expires_at = session.refresh_expires_at;

        if session_revoked || refresh_expires_at <= Utc::now() {
            self.revoke_session_locked(&mut sessions, &session_id);
            return Err("refresh token expired".to_string());
        }

        let access_token = self
            .config
            .token_service
            .issue_access_token(&username, &session_id)?;
        let refresh_token = self.config.token_service.issue_refresh_token();
        sessions.active_refresh_tokens.remove(&refresh_token_hash);
        sessions
            .retired_refresh_tokens
            .insert(refresh_token_hash, session_id.clone());
        if let Some(session) = sessions.sessions.get_mut(&session_id) {
            session.refresh_token_hash = refresh_token.token_hash.clone();
            session.refresh_expires_at = refresh_token.expires_at;
        }
        sessions
            .active_refresh_tokens
            .insert(refresh_token.token_hash.clone(), session_id);

        Ok(AuthResponse {
            access_token: access_token.token,
            refresh_token: refresh_token.token,
            access_token_expires_at: access_token.expires_at,
            username,
        })
    }

    pub async fn logout(&self, request: LogoutRequest) -> Result<(), String> {
        let refresh_token_hash = hash_token(&request.refresh_token);
        let mut sessions = self.sessions.write().await;

        if let Some(session_id) = sessions
            .active_refresh_tokens
            .get(&refresh_token_hash)
            .cloned()
        {
            self.revoke_session_locked(&mut sessions, &session_id);
            return Ok(());
        }

        if let Some(session_id) = sessions
            .retired_refresh_tokens
            .get(&refresh_token_hash)
            .cloned()
        {
            self.revoke_session_locked(&mut sessions, &session_id);
        }

        Ok(())
    }

    pub async fn authenticate_access_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedUser, String> {
        let claims = self.config.token_service.decode_access_token(token)?;
        let sessions = self.sessions.read().await;

        let Some(session) = sessions.sessions.get(&claims.sid) else {
            return Err("unknown session".to_string());
        };

        if session.revoked {
            return Err("session revoked".to_string());
        }

        if session.username != claims.sub {
            return Err("invalid session subject".to_string());
        }

        let user_id = self.lookup_user_id(&session.username).await?;

        Ok(AuthenticatedUser {
            user_id,
            username: session.username.clone(),
            session_id: claims.sid,
        })
    }

    async fn lookup_user_id(&self, username: &str) -> Result<String, String> {
        let username = username.to_string();
        run_db(self.pool.clone(), move |conn| {
            orm::schema::users::table
                .filter(orm::schema::users::username.eq(&username))
                .select(orm::schema::users::id)
                .first::<String>(conn)
                .map_err(|e| format!("user lookup failed: {e}"))
        })
        .await
    }

    pub async fn active_session_count_for(&self, username: &str) -> usize {
        let sessions = self.sessions.read().await;

        sessions
            .sessions
            .values()
            .filter(|session| !session.revoked && session.username == username)
            .count()
    }

    async fn create_session(&self, username: &str) -> Result<AuthResponse, String> {
        let session_id = Uuid::new_v4().to_string();
        let access_token = self
            .config
            .token_service
            .issue_access_token(username, &session_id)?;
        let refresh_token = self.config.token_service.issue_refresh_token();

        let mut sessions = self.sessions.write().await;
        sessions
            .active_refresh_tokens
            .insert(refresh_token.token_hash.clone(), session_id.clone());
        sessions.sessions.insert(
            session_id,
            SessionRecord {
                username: username.to_string(),
                refresh_token_hash: refresh_token.token_hash.clone(),
                refresh_expires_at: refresh_token.expires_at,
                revoked: false,
            },
        );

        Ok(AuthResponse {
            access_token: access_token.token,
            refresh_token: refresh_token.token,
            access_token_expires_at: access_token.expires_at,
            username: username.to_string(),
        })
    }

    fn revoke_session_locked(&self, sessions: &mut SessionState, session_id: &str) {
        let Some(session) = sessions.sessions.get_mut(session_id) else {
            return;
        };

        session.revoked = true;
        sessions
            .active_refresh_tokens
            .remove(&session.refresh_token_hash);
        sessions
            .retired_refresh_tokens
            .insert(session.refresh_token_hash.clone(), session_id.to_string());
    }
}
