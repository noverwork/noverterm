use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
pub struct JsonTokenStore {
    path: PathBuf,
    file_lock: Arc<Mutex<()>>,
}

impl JsonTokenStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            file_lock: Arc::new(Mutex::new(())),
        }
    }
}

impl SecureTokenStore for JsonTokenStore {
    fn load(&self) -> Result<Option<StoredAuthTokens>, String> {
        let _guard = self.file_lock.lock().map_err(|error| error.to_string())?;

        match fs::read_to_string(&self.path) {
            Ok(serialized_tokens) => serde_json::from_str(&serialized_tokens)
                .map(Some)
                .map_err(|error| error.to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save(&self, tokens: &StoredAuthTokens) -> Result<(), String> {
        let _guard = self.file_lock.lock().map_err(|error| error.to_string())?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let serialized_tokens = serde_json::to_string(tokens).map_err(|error| error.to_string())?;
        fs::write(&self.path, serialized_tokens).map_err(|error| error.to_string())
    }

    fn clear(&self) -> Result<(), String> {
        let _guard = self.file_lock.lock().map_err(|error| error.to_string())?;

        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
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

#[cfg(test)]
mod tests {
    use super::{JsonTokenStore, SecureTokenStore, StoredAuthTokens};

    #[test]
    fn json_token_store_round_trips_and_clears_tokens() {
        let path = std::env::temp_dir().join(format!(
            "noverterm-auth-tokens-{}.json",
            uuid::Uuid::new_v4()
        ));
        let store = JsonTokenStore::new(path.clone());
        let tokens = StoredAuthTokens {
            access_token: "access-token".to_string(),
            refresh_token: "refresh-token".to_string(),
            username: "alice".to_string(),
        };

        store.save(&tokens).expect("tokens should save");

        let loaded = store.load().expect("tokens should load");
        assert!(loaded.is_some());
        assert_eq!(loaded.expect("tokens should exist").username, "alice");

        store.clear().expect("tokens should clear");
        assert!(store.load().expect("load after clear should succeed").is_none());
    }
}
