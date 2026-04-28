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
    pub ssh_key_id: Option<String>,
    pub group_id: Option<String>,
    pub auth: Option<SshHostAuthMaterial>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HostGroupRecord {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SshKeyRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
}
