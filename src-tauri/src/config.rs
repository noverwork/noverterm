use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub values: HashMap<String, String>,
}

pub struct SettingsManager {
    path: PathBuf,
    settings: Mutex<AppSettings>,
}

impl SettingsManager {
    pub fn new(path: PathBuf) -> Self {
        let settings = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            AppSettings::default()
        };

        Self {
            path,
            settings: Mutex::new(settings),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.settings.lock().unwrap().values.get(key).cloned()
    }

    pub fn set(&self, key: String, value: String) -> Result<(), String> {
        {
            let mut settings = self.settings.lock().unwrap();
            settings.values.insert(key, value);
        }
        self.save()
    }

    pub fn all(&self) -> HashMap<String, String> {
        self.settings.lock().unwrap().values.clone()
    }

    fn save(&self) -> Result<(), String> {
        let settings = self.settings.lock().unwrap();
        let content = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        fs::write(&self.path, content).map_err(|e| e.to_string())
    }
}
