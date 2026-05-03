use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    fingerprint_key: Vec<u8>,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub sid: String,
    pub exp: usize,
    pub iat: usize,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: String,
    pub sid: String,
    pub exp: usize,
    pub iat: usize,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordResetClaims {
    pub sub: String,
    pub pwd: String,
    pub exp: usize,
    pub iat: usize,
    pub typ: String,
}

#[derive(Debug, Clone)]
pub struct PasswordResetToken {
    pub token: String,
}

impl TokenService {
    pub fn new(secret: impl AsRef<[u8]>, access_ttl: Duration, refresh_ttl: Duration) -> Self {
        let secret = secret.as_ref().to_vec();

        Self {
            encoding_key: EncodingKey::from_secret(&secret),
            decoding_key: DecodingKey::from_secret(&secret),
            fingerprint_key: secret,
            access_ttl,
            refresh_ttl,
        }
    }

    pub fn issue_access_token(
        &self,
        username: &str,
        session_id: &str,
    ) -> Result<AccessToken, String> {
        let issued_at = Utc::now();
        let expires_at = issued_at + self.access_ttl;
        let claims = AccessTokenClaims {
            sub: username.to_string(),
            sid: session_id.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: issued_at.timestamp() as usize,
            typ: "access".to_string(),
        };

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|error| format!("failed to encode access token: {error}"))?;

        Ok(AccessToken { token, expires_at })
    }

    pub fn issue_refresh_token(&self, email: &str, session_id: &str) -> RefreshToken {
        let issued_at = Utc::now();
        let expires_at = issued_at + self.refresh_ttl;
        let claims = RefreshTokenClaims {
            sub: email.to_string(),
            sid: session_id.to_string(),
            exp: expires_at.timestamp() as usize,
            iat: issued_at.timestamp() as usize,
            typ: "refresh".to_string(),
        };

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .expect("failed to encode refresh token");

        RefreshToken { token }
    }

    pub fn decode_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let data = decode::<RefreshTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|error| format!("invalid refresh token: {error}"))?;

        if data.claims.typ != "refresh" {
            return Err("invalid refresh token type".to_string());
        }

        Ok(data.claims)
    }

    pub fn decode_access_token(&self, token: &str) -> Result<AccessTokenClaims, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|error| format!("invalid access token: {error}"))?;

        if data.claims.typ != "access" {
            return Err("invalid access token type".to_string());
        }

        Ok(data.claims)
    }

    pub fn issue_password_reset_token(
        &self,
        email: &str,
        password_hash: &str,
        ttl: Duration,
    ) -> Result<PasswordResetToken, String> {
        let issued_at = Utc::now();
        let expires_at = issued_at + ttl;
        let claims = PasswordResetClaims {
            sub: email.to_string(),
            pwd: self.password_fingerprint(password_hash),
            exp: expires_at.timestamp() as usize,
            iat: issued_at.timestamp() as usize,
            typ: "password_reset".to_string(),
        };

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|error| format!("failed to encode password reset token: {error}"))?;

        Ok(PasswordResetToken { token })
    }

    pub fn decode_password_reset_token(&self, token: &str) -> Result<PasswordResetClaims, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let data = decode::<PasswordResetClaims>(token, &self.decoding_key, &validation)
            .map_err(|error| format!("invalid password reset token: {error}"))?;

        if data.claims.typ != "password_reset" {
            return Err("invalid password reset token type".to_string());
        }

        Ok(data.claims)
    }

    pub fn password_fingerprint(&self, password_hash: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.fingerprint_key)
            .expect("HMAC accepts keys of any length");
        mac.update(password_hash.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hashing should not fail")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
