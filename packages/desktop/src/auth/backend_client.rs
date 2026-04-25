use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct BackendClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize)]
struct LoginPayload<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct RefreshPayload<'a> {
    refresh_token: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct LogoutPayload<'a> {
    refresh_token: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackendAuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: Option<String>,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendHostUpsertInput {
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendKeyUpsertInput {
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BackendConnectMaterial {
    pub issuance_id: String,
    pub host_id: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub issued_for_username: String,
    pub issued_for_session_id: String,
    pub expires_at: String,
    pub auth: BackendConnectAuthMaterial,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackendConnectAuthMaterial {
    Password {
        password: String,
    },
    PublicKey {
        private_key: String,
        passphrase: Option<String>,
    },
    PublicKeyAndPassword {
        private_key: String,
        passphrase: Option<String>,
        password: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct BootstrapSmokeResponse {
    pub message: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendClientError {
    Unauthorized(String),
    Transport(String),
}

impl BackendClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .connect_timeout(std::time::Duration::from_secs(3))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<BackendAuthResponse, BackendClientError> {
        let response = self
            .http
            .post(format!("{}/auth/login", self.base_url))
            .json(&LoginPayload { email, password })
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn refresh(
        &self,
        refresh_token: &str,
    ) -> Result<BackendAuthResponse, BackendClientError> {
        let response = self
            .http
            .post(format!("{}/auth/refresh", self.base_url))
            .json(&RefreshPayload { refresh_token })
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<(), BackendClientError> {
        let response = self
            .http
            .post(format!("{}/auth/logout", self.base_url))
            .json(&LogoutPayload { refresh_token })
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(BackendClientError::Unauthorized(
                read_error_body(response).await,
            )),
            _ => Err(BackendClientError::Transport(
                read_error_body(response).await,
            )),
        }
    }

    pub async fn bootstrap_smoke(
        &self,
        access_token: &str,
    ) -> Result<BootstrapSmokeResponse, BackendClientError> {
        let response = self
            .http
            .get(format!("{}/bootstrap/smoke", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn issue_connect_material(
        &self,
        access_token: &str,
        host_id: &str,
    ) -> Result<BackendConnectMaterial, BackendClientError> {
        let response = self
            .http
            .post(format!(
                "{}/bootstrap/connect/{host_id}/issue",
                self.base_url
            ))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn list_settings(
        &self,
        access_token: &str,
    ) -> Result<Vec<shared::Setting>, BackendClientError> {
        let response = self
            .http
            .get(format!("{}/bootstrap/settings", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn create_setting(
        &self,
        access_token: &str,
        setting: &shared::Setting,
    ) -> Result<shared::Setting, BackendClientError> {
        let response = self
            .http
            .post(format!("{}/bootstrap/settings", self.base_url))
            .bearer_auth(access_token)
            .json(setting)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response_with_statuses(response, &[StatusCode::CREATED]).await
    }

    pub async fn update_setting(
        &self,
        access_token: &str,
        setting: &shared::Setting,
    ) -> Result<shared::Setting, BackendClientError> {
        let response = self
            .http
            .put(format!(
                "{}/bootstrap/settings/{}",
                self.base_url, setting.key
            ))
            .bearer_auth(access_token)
            .json(setting)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn list_hosts(
        &self,
        access_token: &str,
    ) -> Result<Vec<shared::SshHostRecord>, BackendClientError> {
        let response = self
            .http
            .get(format!("{}/bootstrap/hosts", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn create_host(
        &self,
        access_token: &str,
        input: &BackendHostUpsertInput,
    ) -> Result<shared::SshHostRecord, BackendClientError> {
        let response = self
            .http
            .post(format!("{}/bootstrap/hosts", self.base_url))
            .bearer_auth(access_token)
            .json(input)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response_with_statuses(response, &[StatusCode::CREATED]).await
    }

    pub async fn update_host(
        &self,
        access_token: &str,
        id: &str,
        input: &BackendHostUpsertInput,
    ) -> Result<shared::SshHostRecord, BackendClientError> {
        let response = self
            .http
            .put(format!("{}/bootstrap/hosts/{id}", self.base_url))
            .bearer_auth(access_token)
            .json(input)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn delete_host(
        &self,
        access_token: &str,
        id: &str,
    ) -> Result<(), BackendClientError> {
        let response = self
            .http
            .delete(format!("{}/bootstrap/hosts/{id}", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_empty_response(response, &[StatusCode::NO_CONTENT]).await
    }

    pub async fn list_keys(
        &self,
        access_token: &str,
    ) -> Result<Vec<shared::SshKeyRecord>, BackendClientError> {
        let response = self
            .http
            .get(format!("{}/bootstrap/keys", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn create_key(
        &self,
        access_token: &str,
        input: &BackendKeyUpsertInput,
    ) -> Result<shared::SshKeyRecord, BackendClientError> {
        let response = self
            .http
            .post(format!("{}/bootstrap/keys", self.base_url))
            .bearer_auth(access_token)
            .json(input)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response_with_statuses(response, &[StatusCode::CREATED]).await
    }

    pub async fn update_key(
        &self,
        access_token: &str,
        id: &str,
        input: &BackendKeyUpsertInput,
    ) -> Result<shared::SshKeyRecord, BackendClientError> {
        let response = self
            .http
            .put(format!("{}/bootstrap/keys/{id}", self.base_url))
            .bearer_auth(access_token)
            .json(input)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_json_response(response).await
    }

    pub async fn delete_key(&self, access_token: &str, id: &str) -> Result<(), BackendClientError> {
        let response = self
            .http
            .delete(format!("{}/bootstrap/keys/{id}", self.base_url))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string()))?;

        parse_empty_response(response, &[StatusCode::NO_CONTENT]).await
    }
}

async fn parse_json_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, BackendClientError> {
    parse_json_response_with_statuses(response, &[StatusCode::OK]).await
}

async fn parse_json_response_with_statuses<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    success_statuses: &[StatusCode],
) -> Result<T, BackendClientError> {
    match response.status() {
        status if success_statuses.contains(&status) => response
            .json::<T>()
            .await
            .map_err(|error| BackendClientError::Transport(error.to_string())),
        StatusCode::UNAUTHORIZED => Err(BackendClientError::Unauthorized(
            read_error_body(response).await,
        )),
        _ => Err(BackendClientError::Transport(
            read_error_body(response).await,
        )),
    }
}

async fn parse_empty_response(
    response: reqwest::Response,
    success_statuses: &[StatusCode],
) -> Result<(), BackendClientError> {
    match response.status() {
        status if success_statuses.contains(&status) => Ok(()),
        StatusCode::UNAUTHORIZED => Err(BackendClientError::Unauthorized(
            read_error_body(response).await,
        )),
        _ => Err(BackendClientError::Transport(
            read_error_body(response).await,
        )),
    }
}

async fn read_error_body(response: reqwest::Response) -> String {
    response
        .text()
        .await
        .unwrap_or_else(|error| format!("failed to read backend error body: {error}"))
}
