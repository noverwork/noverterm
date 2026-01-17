use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Group Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroup {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub color: Option<String>,
}

// ============================================================================
// SSH Key Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SshKey {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub public_key: String,
    pub private_key_path: Option<String>,
    pub fingerprint: String,
    pub has_passphrase: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSshKey {
    pub name: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub public_key: String,
    pub private_key_path: Option<String>,
    pub fingerprint: String,
    pub has_passphrase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSshKey {
    pub name: Option<String>,
}

// ============================================================================
// Session Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub group_id: Option<String>,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub key_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSession {
    pub name: String,
    pub group_id: Option<String>,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_method: String,
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSession {
    pub name: Option<String>,
    pub group_id: Option<Option<String>>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub auth_method: Option<String>,
    pub key_id: Option<Option<String>>,
}

// ============================================================================
// Port Forward Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PortForward {
    pub id: String,
    pub session_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub forward_type: String,
    pub local_host: String,
    pub local_port: i64,
    pub remote_host: Option<String>,
    pub remote_port: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePortForward {
    pub session_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub forward_type: String,
    pub local_host: String,
    pub local_port: i64,
    pub remote_host: Option<String>,
    pub remote_port: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePortForward {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub forward_type: Option<String>,
    pub local_host: Option<String>,
    pub local_port: Option<i64>,
    pub remote_host: Option<Option<String>>,
    pub remote_port: Option<i64>,
}
