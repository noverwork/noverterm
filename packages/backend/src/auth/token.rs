use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
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

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub token: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

impl TokenService {
    pub fn new(secret: impl AsRef<[u8]>, access_ttl: Duration, refresh_ttl: Duration) -> Self {
        let secret = secret.as_ref().to_vec();

        Self {
            encoding_key: EncodingKey::from_secret(&secret),
            decoding_key: DecodingKey::from_secret(&secret),
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

    pub fn issue_refresh_token(&self) -> RefreshToken {
        let mut random_bytes = [0_u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut random_bytes);
        let token = URL_SAFE_NO_PAD.encode(random_bytes);

        RefreshToken {
            token_hash: hash_token(&token),
            token,
            expires_at: Utc::now() + self.refresh_ttl,
        }
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
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn hash_password(password: &str) -> String {
    hash_token(password)
}
