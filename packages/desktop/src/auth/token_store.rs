use keyring::Entry;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub username: String,
}

pub trait SecureTokenStore: Send + Sync + 'static {
    fn load(&self) -> Result<Option<StoredAuthTokens>, String>;
    fn save(&self, tokens: &StoredAuthTokens) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct KeyringTokenStore {
    service_name: String,
    account_name: String,
}

impl KeyringTokenStore {
    pub fn new(service_name: impl Into<String>, account_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            account_name: account_name.into(),
        }
    }

    fn entry(&self) -> Result<Entry, String> {
        Entry::new(&self.service_name, &self.account_name).map_err(|error| error.to_string())
    }
}

impl SecureTokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<StoredAuthTokens>, String> {
        let entry = self.entry()?;

        match entry.get_password() {
            Ok(serialized_tokens) => serde_json::from_str(&serialized_tokens)
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save(&self, tokens: &StoredAuthTokens) -> Result<(), String> {
        let entry = self.entry()?;
        let serialized_tokens = serde_json::to_string(tokens).map_err(|error| error.to_string())?;

        entry
            .set_password(&serialized_tokens)
            .map_err(|error| error.to_string())
    }

    fn clear(&self) -> Result<(), String> {
        let entry = self.entry()?;

        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct MemoryTokenStore {
    tokens: std::sync::Arc<std::sync::Mutex<Option<StoredAuthTokens>>>,
}

#[cfg(test)]
impl SecureTokenStore for MemoryTokenStore {
    fn load(&self) -> Result<Option<StoredAuthTokens>, String> {
        self.tokens
            .lock()
            .map(|tokens| tokens.clone())
            .map_err(|error| error.to_string())
    }

    fn save(&self, tokens: &StoredAuthTokens) -> Result<(), String> {
        self.tokens
            .lock()
            .map(|mut current| {
                *current = Some(tokens.clone());
            })
            .map_err(|error| error.to_string())
    }

    fn clear(&self) -> Result<(), String> {
        self.tokens
            .lock()
            .map(|mut current| {
                *current = None;
            })
            .map_err(|error| error.to_string())
    }
}
