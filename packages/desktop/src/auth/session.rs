use tokio::sync::RwLock;

use super::backend_client::{
    BackendClient, BackendClientError, BackendHostUpsertInput, BackendKeyUpsertInput,
};
use std::path::PathBuf;

use super::token_store::{JsonTokenStore, SecureTokenStore, StoredAuthTokens};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type, PartialEq, Eq)]
pub struct AuthBootstrapStatus {
    pub email: String,
    pub bootstrap_message: String,
}

pub type DesktopAuthManager = AuthManager<JsonTokenStore>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BootstrapMetadata {
    pub settings: Vec<shared::Setting>,
    pub hosts: Vec<shared::SshHostRecord>,
    pub keys: Vec<shared::SshKeyRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SaveConnectionInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub passphrase: Option<String>,
    pub existing_key_id: Option<String>,
}

pub struct AuthManager<S: SecureTokenStore> {
    client: BackendClient,
    store: S,
    cached_tokens: RwLock<Option<StoredAuthTokens>>,
}

impl<S: SecureTokenStore> AuthManager<S> {
    pub fn new(client: BackendClient, store: S) -> Self {
        Self {
            client,
            store,
            cached_tokens: RwLock::new(None),
        }
    }

    pub async fn login(
        &self,
        email: String,
        password: String,
    ) -> Result<AuthBootstrapStatus, String> {
        let auth_response = self
            .client
            .login(&email, &password)
            .await
            .map_err(map_client_error)?;
        let tokens = StoredAuthTokens {
            access_token: auth_response.access_token,
            refresh_token: auth_response.refresh_token,
            email: auth_response.email,
        };

        let bootstrap_status = self.complete_bootstrap(tokens).await?;
        Ok(bootstrap_status)
    }

    pub async fn restore(&self) -> Result<Option<AuthBootstrapStatus>, String> {
        let stored_tokens = if let Some(tokens) = self.cached_tokens.read().await.clone() {
            Some(tokens)
        } else {
            self.store.load()?
        };

        let Some(tokens) = stored_tokens else {
            return Ok(None);
        };

        match self.bootstrap_with_refresh(tokens).await {
            Ok(status) => Ok(Some(status)),
            Err(_) => {
                self.clear_local_session().await?;
                Ok(None)
            }
        }
    }

    pub async fn logout(&self) -> Result<(), String> {
        let tokens = if let Some(tokens) = self.cached_tokens.read().await.clone() {
            Some(tokens)
        } else {
            self.store.load()?
        };

        if let Some(tokens) = tokens {
            match self.client.logout(&tokens.refresh_token).await {
                Ok(()) | Err(BackendClientError::Unauthorized(_)) => {}
                Err(BackendClientError::Transport(error)) => return Err(error),
            }
        }

        self.clear_local_session().await
    }

    async fn complete_bootstrap(
        &self,
        tokens: StoredAuthTokens,
    ) -> Result<AuthBootstrapStatus, String> {
        self.persist_tokens(&tokens).await?;

        match self.client.bootstrap_smoke(&tokens.access_token).await {
            Ok(response) => Ok(AuthBootstrapStatus {
                email: response.email,
                bootstrap_message: response.message,
            }),
            Err(BackendClientError::Unauthorized(_)) => {
                match self.bootstrap_with_refresh(tokens).await {
                    Ok(status) => Ok(status),
                    Err(error) => {
                        self.clear_local_session().await?;
                        Err(error)
                    }
                }
            }
            Err(BackendClientError::Transport(error)) => {
                self.clear_local_session().await?;
                Err(error)
            }
        }
    }

    async fn bootstrap_with_refresh(
        &self,
        tokens: StoredAuthTokens,
    ) -> Result<AuthBootstrapStatus, String> {
        match self.client.bootstrap_smoke(&tokens.access_token).await {
            Ok(response) => {
                self.persist_tokens(&tokens).await?;
                Ok(AuthBootstrapStatus {
                    email: response.email,
                    bootstrap_message: response.message,
                })
            }
            Err(BackendClientError::Unauthorized(_)) => {
                let refreshed = self
                    .client
                    .refresh(&tokens.refresh_token)
                    .await
                    .map_err(map_client_error)?;
                let rotated_tokens = StoredAuthTokens {
                    access_token: refreshed.access_token,
                    refresh_token: refreshed.refresh_token,
                    email: refreshed.email,
                };

                self.persist_tokens(&rotated_tokens).await?;
                let response = self
                    .client
                    .bootstrap_smoke(&rotated_tokens.access_token)
                    .await
                    .map_err(map_client_error)?;

                Ok(AuthBootstrapStatus {
                    email: response.email,
                    bootstrap_message: response.message,
                })
            }
            Err(BackendClientError::Transport(error)) => Err(error),
        }
    }

    async fn persist_tokens(&self, tokens: &StoredAuthTokens) -> Result<(), String> {
        self.store.save(tokens)?;
        *self.cached_tokens.write().await = Some(tokens.clone());
        Ok(())
    }

    async fn clear_local_session(&self) -> Result<(), String> {
        self.store.clear()?;
        *self.cached_tokens.write().await = None;
        Ok(())
    }

    async fn require_tokens(&self) -> Result<StoredAuthTokens, String> {
        let tokens = if let Some(tokens) = self.cached_tokens.read().await.clone() {
            Some(tokens)
        } else {
            self.store.load()?
        };

        tokens.ok_or_else(|| "not authenticated".to_string())
    }

    async fn fresh_tokens(&self) -> Result<StoredAuthTokens, String> {
        let tokens = self.require_tokens().await?;
        self.bootstrap_with_refresh(tokens).await?;

        self.cached_tokens
            .read()
            .await
            .clone()
            .ok_or_else(|| "not authenticated".to_string())
    }

    pub async fn upsert_setting(
        &self,
        setting: shared::Setting,
    ) -> Result<shared::Setting, String> {
        let tokens = self.fresh_tokens().await?;

        match self
            .client
            .update_setting(&tokens.access_token, &setting)
            .await
        {
            Ok(saved) => Ok(saved),
            Err(BackendClientError::Transport(_)) => self
                .client
                .create_setting(&tokens.access_token, &setting)
                .await
                .map_err(map_client_error),
            Err(error) => Err(map_client_error(error)),
        }
    }

    pub async fn save_connection(
        &self,
        connection: SaveConnectionInput,
    ) -> Result<shared::SshHostRecord, String> {
        let tokens = self.fresh_tokens().await?;
        let trimmed_password = connection
            .password
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let trimmed_private_key = connection
            .private_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let trimmed_passphrase = connection
            .passphrase
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let ssh_key_id = if let Some(private_key) = trimmed_private_key {
            let key_input = BackendKeyUpsertInput {
                name: format!("{} key", connection.name),
                kind: "inline".to_string(),
                fingerprint: None,
                encrypted_private_key: private_key,
                encrypted_passphrase: trimmed_passphrase,
            };

            Some(
                if let Some(existing_key_id) = connection.existing_key_id.clone() {
                    self.client
                        .update_key(&tokens.access_token, &existing_key_id, &key_input)
                        .await
                        .map_err(map_client_error)?
                        .id
                } else {
                    self.client
                        .create_key(&tokens.access_token, &key_input)
                        .await
                        .map_err(map_client_error)?
                        .id
                },
            )
        } else {
            connection.existing_key_id.clone()
        };

        if trimmed_password.is_none() && ssh_key_id.is_none() {
            return Err("password or private key is required".to_string());
        }

        let host_input = BackendHostUpsertInput {
            name: connection.name,
            host: connection.host,
            port: connection.port,
            username: connection.username,
            ssh_key_id,
            encrypted_password: trimmed_password,
        };

        if let Some(id) = connection.id {
            self.client
                .update_host(&tokens.access_token, &id, &host_input)
                .await
                .map_err(map_client_error)
        } else {
            self.client
                .create_host(&tokens.access_token, &host_input)
                .await
                .map_err(map_client_error)
        }
    }

    pub async fn delete_connection(&self, id: String) -> Result<(), String> {
        let tokens = self.fresh_tokens().await?;

        self.client
            .delete_host(&tokens.access_token, &id)
            .await
            .map_err(map_client_error)
    }

    pub async fn delete_key(&self, id: String) -> Result<(), String> {
        let tokens = self.fresh_tokens().await?;

        self.client
            .delete_key(&tokens.access_token, &id)
            .await
            .map_err(map_client_error)
    }

    pub async fn load_settings(&self) -> Result<Vec<shared::Setting>, String> {
        let tokens = self.fresh_tokens().await?;
        self.client
            .list_settings(&tokens.access_token)
            .await
            .map_err(map_client_error)
    }

    pub async fn load_hosts(&self) -> Result<Vec<shared::SshHostRecord>, String> {
        let tokens = self.fresh_tokens().await?;
        self.client
            .list_hosts(&tokens.access_token)
            .await
            .map_err(map_client_error)
    }

    pub async fn load_keys(&self) -> Result<Vec<shared::SshKeyRecord>, String> {
        let tokens = self.fresh_tokens().await?;
        self.client
            .list_keys(&tokens.access_token)
            .await
            .map_err(map_client_error)
    }

    pub async fn load_bootstrap_metadata(&self) -> Result<BootstrapMetadata, String> {
        let (settings, hosts, keys) =
            tokio::join!(self.load_settings(), self.load_hosts(), self.load_keys(),);

        Ok(BootstrapMetadata {
            settings: settings?,
            hosts: hosts?,
            keys: keys?,
        })
    }
}

impl AuthManager<JsonTokenStore> {
    pub fn from_backend_url(base_url: String, store_path: PathBuf) -> Self {
        Self::new(
            BackendClient::new(base_url),
            JsonTokenStore::new(store_path),
        )
    }
}

fn map_client_error(error: BackendClientError) -> String {
    match error {
        BackendClientError::Unauthorized(message) | BackendClientError::Transport(message) => {
            message
        }
    }
}
