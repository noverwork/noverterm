use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SshHostRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub auth_mode: String,
    pub ssh_key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SshKeyRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
}
