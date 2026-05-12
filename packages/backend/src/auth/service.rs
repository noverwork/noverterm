use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewUser, User};
use orm::schema::users;
use shared::{AuthResponse, ForgotPasswordRequest, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest, ResetPasswordRequest};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::token::{hash_password, verify_password, TokenService};
use crate::db::{run_db, DbPool};
use crate::email::{PasswordResetEmailConfig, PasswordResetMailer};

#[derive(Clone)]
pub struct AuthConfig {
    token_service: TokenService,
    password_reset_ttl: chrono::Duration,
    password_reset_throttle: chrono::Duration,
    password_reset_url: String,
    password_reset_mailer: Option<PasswordResetMailer>,
}

#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
    pool: DbPool,
    password_reset_attempts: Arc<Mutex<HashMap<String, chrono::DateTime<Utc>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: String,
    pub session_id: String,
}

impl AuthConfig {
    pub fn new(secret: String) -> Self {
        Self {
            token_service: TokenService::new(
                secret,
                chrono::Duration::minutes(15),
                chrono::Duration::days(30),
            ),
            password_reset_ttl: chrono::Duration::hours(1),
            password_reset_throttle: chrono::Duration::minutes(5),
            password_reset_url: "http://localhost:1420/reset-password".to_string(),
            password_reset_mailer: None,
        }
    }

    pub fn with_password_reset_url(mut self, password_reset_url: String) -> Self {
        self.password_reset_url = password_reset_url;
        self
    }

    pub fn with_password_reset_email(mut self, config: PasswordResetEmailConfig) -> Self {
        self.password_reset_mailer = Some(PasswordResetMailer::new(config));
        self
    }
}

impl AuthService {
    pub fn new(config: AuthConfig, pool: DbPool) -> Self {
        Self {
            config,
            pool,
            password_reset_attempts: Arc::new(Mutex::new(HashMap::new())),
        }
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
        self.ensure_token_is_current(&email, claims.iat).await?;

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

    pub async fn request_password_reset(
        &self,
        request: ForgotPasswordRequest,
    ) -> Result<(), String> {
        let email = request.email.trim().to_string();
        if email.is_empty() {
            tracing::info!("password reset requested with empty email; no email sent");
            return Ok(());
        }

        tracing::info!(email = %email, "password reset requested");

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
            tracing::info!(email = %email, "password reset requested for unknown account; no email sent");
            return Ok(());
        };

        tracing::info!(email = %user.email, "password reset account matched; checking throttle");
        if self.is_password_reset_throttled(&user.email).await {
            tracing::info!(email = %user.email, "password reset request throttled for existing account; no email sent");
            return Ok(());
        }

        tracing::info!(email = %user.email, "issuing password reset token");
        let reset_token = self.config.token_service.issue_password_reset_token(
            &user.email,
            &user.password_hash,
            self.config.password_reset_ttl,
        )?;
        let reset_link = self.password_reset_link(&reset_token.token);
        if let Some(mailer) = self.config.password_reset_mailer.as_ref() {
            tracing::info!(email = %user.email, "sending password reset email");
            if let Err(error) = mailer.send_reset_link(&user.email, &reset_link).await {
                tracing::error!(%error, email = %user.email, "failed to send password reset email");
            } else {
                tracing::info!(email = %user.email, "password reset email sent");
            }
        } else {
            tracing::warn!(email = %user.email, "password reset mailer is not configured; no email sent");
        }

        Ok(())
    }

    pub async fn reset_password(&self, request: ResetPasswordRequest) -> Result<(), String> {
        if request.password.len() < 6 {
            return Err("password must be at least 6 characters".to_string());
        }

        let claims = self
            .config
            .token_service
            .decode_password_reset_token(&request.token)?;
        let email = claims.sub.clone();
        let user: Option<User> = run_db(self.pool.clone(), move |conn| {
            users::table
                .filter(users::email.eq(&email))
                .first::<User>(conn)
                .optional()
                .map_err(|e| format!("database error: {e}"))
        })
        .await?;

        let Some(user) = user else {
            return Err("invalid password reset token".to_string());
        };

        let expected_fingerprint = self
            .config
            .token_service
            .password_fingerprint(&user.password_hash);
        if claims.pwd != expected_fingerprint {
            return Err("invalid password reset token".to_string());
        }

        let password_hash = hash_password(&request.password);
        let user_id = user.id;
        let updated_at = Utc::now().naive_utc();
        run_db(self.pool.clone(), move |conn| {
            diesel::update(users::table.filter(users::id.eq(&user_id)))
                .set((
                    users::password_hash.eq(password_hash),
                    users::updated_at.eq(updated_at),
                ))
                .execute(conn)
                .map_err(|e| format!("database error: {e}"))
        })
        .await?;

        Ok(())
    }

    pub async fn authenticate_access_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedUser, String> {
        let claims = self.config.token_service.decode_access_token(token)?;

        let user_id = self.lookup_user_id(&claims.sub).await?;
        self.ensure_token_is_current(&claims.sub, claims.iat)
            .await?;

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

    async fn ensure_token_is_current(&self, email: &str, issued_at: usize) -> Result<(), String> {
        let email = email.to_string();
        let user: User = run_db(self.pool.clone(), move |conn| {
            users::table
                .filter(users::email.eq(&email))
                .first::<User>(conn)
                .map_err(|e| format!("user lookup failed: {e}"))
        })
        .await?;

        if (issued_at as i64) < user.updated_at.and_utc().timestamp() {
            return Err("token expired after password change".to_string());
        }

        Ok(())
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

    fn password_reset_link(&self, token: &str) -> String {
        let separator = if self.config.password_reset_url.contains('?') {
            '&'
        } else {
            '?'
        };
        format!("{}{separator}token={token}", self.config.password_reset_url)
    }

    async fn is_password_reset_throttled(&self, email: &str) -> bool {
        let now = Utc::now();
        let key = email.trim().to_lowercase();
        let mut attempts = self.password_reset_attempts.lock().await;

        attempts.retain(|_, last_attempt| {
            now.signed_duration_since(*last_attempt) < self.config.password_reset_throttle
        });

        if let Some(last_attempt) = attempts.get(&key) {
            if now.signed_duration_since(*last_attempt) < self.config.password_reset_throttle {
                return true;
            }
        }

        attempts.insert(key, now);
        false
    }

    #[cfg(test)]
    pub(crate) async fn has_password_reset_attempt_for_test(&self, email: &str) -> bool {
        let key = email.trim().to_lowercase();
        self.password_reset_attempts.lock().await.contains_key(&key)
    }
}
