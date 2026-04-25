use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewUser, User};
use orm::schema::users;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::token::{hash_password, verify_password, TokenService};
use crate::db::{run_db, DbPool};

#[derive(Clone)]
pub struct AuthConfig {
    token_service: TokenService,
}

#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
    pool: DbPool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
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
    pub email: String,
}

impl AuthConfig {
    pub fn new(secret: String) -> Self {
        Self {
            token_service: TokenService::new(
                secret,
                chrono::Duration::minutes(15),
                chrono::Duration::days(30),
            ),
        }
    }
}

impl AuthService {
    pub fn new(config: AuthConfig, pool: DbPool) -> Self {
        Self { config, pool }
    }

    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, String> {
        let email = request.email.clone();
        let check_email = email.clone();
        let existing: Result<Option<User>, String> = run_db(self.pool.clone(), move |conn| {
            users::table
                .filter(users::email.eq(&check_email))
                .first::<User>(conn)
                .optional()
                .map_err(|e| format!("database error: {e}"))
        })
        .await;

        if existing?.is_some() {
            return Err("email already exists".to_string());
        }

        let password_hash = hash_password(&request.password);
        let now = Utc::now().naive_utc();
        let new_user = NewUser {
            id: Uuid::new_v4().to_string(),
            email: request.email.clone(),
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

        self.create_session(&email).await
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, String> {
        let email = request.email.clone();
        let check_email = email.clone();
        let user: Option<User> = run_db(self.pool.clone(), move |conn| {
            users::table
                .filter(users::email.eq(&check_email))
                .first::<User>(conn)
                .optional()
                .map_err(|e| format!("database error: {e}"))
        })
        .await?;

        let Some(user) = user else {
            return Err("invalid credentials".to_string());
        };

        if !verify_password(&request.password, &user.password_hash) {
            return Err("invalid credentials".to_string());
        }

        self.create_session(&email).await
    }

    pub async fn refresh(&self, request: RefreshRequest) -> Result<AuthResponse, String> {
        let claims = self
            .config
            .token_service
            .decode_refresh_token(&request.refresh_token)?;

        let email = claims.sub;
        let session_id = claims.sid;

        let access_token = self
            .config
            .token_service
            .issue_access_token(&email, &session_id)?;
        let refresh_token = self
            .config
            .token_service
            .issue_refresh_token(&email, &session_id);

        Ok(AuthResponse {
            access_token: access_token.token,
            refresh_token: refresh_token.token,
            access_token_expires_at: access_token.expires_at,
            email,
        })
    }

    pub async fn logout(&self, _request: LogoutRequest) -> Result<(), String> {
        Ok(())
    }

    pub async fn authenticate_access_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedUser, String> {
        let claims = self.config.token_service.decode_access_token(token)?;

        let user_id = self.lookup_user_id(&claims.sub).await?;

        Ok(AuthenticatedUser {
            user_id,
            email: claims.sub,
            session_id: claims.sid,
        })
    }

    async fn lookup_user_id(&self, email: &str) -> Result<String, String> {
        let email = email.to_string();
        run_db(self.pool.clone(), move |conn| {
            orm::schema::users::table
                .filter(orm::schema::users::email.eq(&email))
                .select(orm::schema::users::id)
                .first::<String>(conn)
                .map_err(|e| format!("user lookup failed: {e}"))
        })
        .await
    }

    async fn create_session(&self, email: &str) -> Result<AuthResponse, String> {
        let session_id = Uuid::new_v4().to_string();
        let access_token = self
            .config
            .token_service
            .issue_access_token(email, &session_id)?;
        let refresh_token = self
            .config
            .token_service
            .issue_refresh_token(email, &session_id);

        Ok(AuthResponse {
            access_token: access_token.token,
            refresh_token: refresh_token.token,
            access_token_expires_at: access_token.expires_at,
            email: email.to_string(),
        })
    }
}
