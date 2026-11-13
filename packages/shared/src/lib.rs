use serde::{Deserialize, Serialize};

// ============================================================================
// Core Domain Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHostRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub ssh_key_id: Option<String>,
    pub group_id: Option<String>,
    pub jump_host_id: Option<String>,
    pub auth: Option<SshHostAuthMaterial>,
    pub ai_access: AiAccess,
}

/// What MCP clients (AI agents) may do on a host without asking the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiAccess {
    /// Show the approval dialog for every call.
    Ask,
    /// Run without asking.
    Allow,
    /// Refuse without asking.
    Deny,
}

impl AiAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    /// The database column only holds the three values (CHECK constraint);
    /// anything else asks, the safe default.
    pub fn parse(value: &str) -> Self {
        match value {
            "allow" => Self::Allow,
            "deny" => Self::Deny,
            _ => Self::Ask,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostGroupRecord {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeySecret {
    pub private_key: String,
    pub passphrase: Option<String>,
}

// ============================================================================
// Snippet Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetRecord {
    pub id: String,
    pub host_id: Option<String>,
    pub host_name: String,
    pub title: String,
    pub body: String,
    pub pin_icon: Option<String>,
    pub pin_order: Option<i32>,
    pub pin_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetWriteRequest {
    pub host_id: Option<String>,
    pub title: String,
    pub body: String,
    pub pin_icon: Option<String>,
    pub pin_color: Option<String>,
}

// ============================================================================
// Port Forward Preset Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardMappingInput {
    pub bind_host: String,
    pub bind_port: i32,
    pub target_host: String,
    pub target_port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardRecord {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub host_name: String,
    pub mappings: Vec<PortForwardMappingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardWriteRequest {
    pub name: String,
    pub host_id: String,
    pub mappings: Vec<PortForwardMappingInput>,
}

// ============================================================================
// Link Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRecord {
    pub id: String,
    pub subdomain: String,
    pub target_host: String,
    pub target_port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkWriteRequest {
    pub subdomain: String,
    pub target_host: String,
    pub target_port: i32,
}

/// A Link name as the relay accepts it: a lowercase DNS label, or None.
pub fn link_subdomain(requested: &str) -> Option<String> {
    let name = requested.trim().to_ascii_lowercase();
    let valid = (1..=63).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-');
    valid.then_some(name)
}

#[cfg(test)]
mod tests {
    use super::link_subdomain;

    #[test]
    fn link_subdomains_are_dns_labels() {
        assert_eq!(link_subdomain("MyApp"), Some("myapp".into()));
        assert_eq!(link_subdomain(" my-app-2 "), Some("my-app-2".into()));
        assert_eq!(link_subdomain(""), None);
        assert_eq!(link_subdomain("-app"), None);
        assert_eq!(link_subdomain("app-"), None);
        assert_eq!(link_subdomain("a.b"), None);
        assert_eq!(link_subdomain("a_b"), None);
        assert_eq!(link_subdomain(&"a".repeat(64)), None);
    }
}
