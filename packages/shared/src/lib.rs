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
// Snippet Types
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "snippet-write-request.ts")]
pub struct SnippetWriteRequest {
    pub host_id: String,
    pub title: String,
    pub body: String,
}

// ============================================================================
// Port Forward Preset Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "port-forward-mapping-input.ts")]
pub struct PortForwardMappingInput {
    pub bind_host: String,
    pub bind_port: i32,
    pub target_host: String,
    pub target_port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "port-forward-record.ts")]
pub struct PortForwardRecord {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub host_name: String,
    pub mappings: Vec<PortForwardMappingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, TS)]
#[ts(export, export_to = "port-forward-write-request.ts")]
pub struct PortForwardWriteRequest {
    pub name: String,
    pub host_id: String,
    pub mappings: Vec<PortForwardMappingInput>,
}
