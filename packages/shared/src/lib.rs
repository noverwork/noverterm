use serde::{Deserialize, Serialize};
use specta::Type;
use ts_rs::TS;

// ============================================================================
// Core Domain Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "setting.ts")]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "ssh-host-record.ts")]
pub struct SshHostRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub ssh_key_id: Option<String>,
    pub group_id: Option<String>,
    pub auth: Option<SshHostAuthMaterial>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "host-group-record.ts")]
pub struct HostGroupRecord {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export, export_to = "ssh-host-auth-material.ts")]
pub enum SshHostAuthMaterial {
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "ssh-key-record.ts")]
pub struct SshKeyRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "ssh-key-secret.ts")]
pub struct SshKeySecret {
    pub private_key: String,
    pub passphrase: Option<String>,
}

// ============================================================================
// Auth API Types
// ============================================================================

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "login-request.ts")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "register-request.ts")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "refresh-request.ts")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "logout-request.ts")]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "forgot-password-request.ts")]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "reset-password-request.ts")]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth-response.ts")]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: chrono::DateTime<chrono::Utc>,
    pub email: String,
}

// ============================================================================
// Host API Types
// ============================================================================

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "host-write-request.ts")]
pub struct HostWriteRequest {
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub ssh_key_id: Option<String>,
    pub encrypted_password: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
}

// ============================================================================
// Key API Types
// ============================================================================

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "key-write-request.ts")]
pub struct KeyWriteRequest {
    pub name: String,
    pub kind: String,
    #[ts(optional = nullable)]
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "key-update-request.ts")]
pub struct KeyUpdateRequest {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub fingerprint: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub encrypted_private_key: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub encrypted_passphrase: Option<String>,
}

// ============================================================================
// Host Group API Types
// ============================================================================

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "host-group-write-request.ts")]
pub struct HostGroupWriteRequest {
    pub name: String,
}

// ============================================================================
// Snippet API Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "snippet-record.ts")]
pub struct SnippetRecord {
    pub id: String,
    pub host_id: String,
    pub host_name: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "snippet-write-request.ts")]
pub struct SnippetWriteRequest {
    pub host_id: String,
    pub title: String,
    pub body: String,
}
