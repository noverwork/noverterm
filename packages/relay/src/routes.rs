//! Which subdomain goes to which SSH client, and who may connect.

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use russh::keys::PublicKey;

/// A claimed subdomain and the client holding it.
pub struct Route<T> {
    pub client_id: usize,
    pub target: T,
}

/// Live subdomain claims. Generic so tests can run without an SSH session.
pub struct Routes<T>(DashMap<String, Route<T>>);

impl<T: Clone> Routes<T> {
    pub fn new() -> Self {
        Self(DashMap::new())
    }

    /// First come, first served; false when someone else holds the name.
    pub fn claim(&self, name: &str, client_id: usize, target: T) -> bool {
        match self.0.entry(name.to_string()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(slot) => {
                slot.insert(Route { client_id, target });
                true
            }
        }
    }

    pub fn release(&self, name: &str, client_id: usize) {
        self.0
            .remove_if(name, |_, route| route.client_id == client_id);
    }

    pub fn release_client(&self, client_id: usize) {
        self.0.retain(|_, route| route.client_id != client_id);
    }

    pub fn get(&self, name: &str) -> Option<T> {
        self.0.get(name).map(|route| route.target.clone())
    }
}

pub use shared::link_subdomain as subdomain;

/// The subdomain a Host header points at under `base_domain`.
pub fn subdomain_of_host(host: &str, base_domain: &str) -> Option<String> {
    let host = host
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_end_matches('.');
    let base = base_domain.trim_end_matches('.');
    let label = host
        .len()
        .checked_sub(base.len() + 1)
        .filter(|&cut| host.as_bytes()[cut] == b'.' && host[cut + 1..].eq_ignore_ascii_case(base))
        .map(|cut| &host[..cut])?;
    subdomain(label)
}

/// Whether `key` appears in an OpenSSH authorized_keys file's contents.
/// Option prefixes (`restrict ssh-ed25519 ...`) are skipped, not enforced.
pub fn is_authorized(authorized_keys: &str, key: &PublicKey) -> bool {
    authorized_keys
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let start = line.find("ssh-").or_else(|| line.find("ecdsa-"))?;
            PublicKey::from_openssh(&line[start..]).ok()
        })
        .any(|listed| listed.key_data() == key.key_data())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_A: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIO+E/5RkM7jW3dXgfwoL34t/p7zI2BY9z1kg/xyMJw1T a@x";
    const KEY_B: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAGn5lOEHR6twBaucc97KXmqGG00ad3byneHo1zQ9Jxs b@x";

    #[test]
    fn host_maps_to_subdomain() {
        assert_eq!(
            subdomain_of_host("myapp.example.com", "example.com"),
            Some("myapp".into())
        );
        assert_eq!(
            subdomain_of_host("MyApp.Example.com:443", "example.com"),
            Some("myapp".into())
        );
        assert_eq!(subdomain_of_host("example.com", "example.com"), None);
        assert_eq!(subdomain_of_host("a.b.example.com", "example.com"), None);
        assert_eq!(subdomain_of_host("myapp.evil.com", "example.com"), None);
        assert_eq!(subdomain_of_host("myappexample.com", "example.com"), None);
    }

    #[test]
    fn names_are_first_come_first_served() {
        let routes = Routes::new();
        assert!(routes.claim("app", 1, "one"));
        assert!(!routes.claim("app", 2, "two"));
        routes.release("app", 2);
        assert_eq!(routes.get("app"), Some("one"));
        routes.release_client(1);
        assert_eq!(routes.get("app"), None);
        assert!(routes.claim("app", 2, "two"));
    }

    #[test]
    fn only_listed_keys_are_authorized() {
        let a = PublicKey::from_openssh(KEY_A).unwrap();
        let b = PublicKey::from_openssh(KEY_B).unwrap();
        let file = format!("# owner\n\nrestrict {KEY_A}\ngarbage line\n");
        assert!(is_authorized(&file, &a));
        assert!(!is_authorized(&file, &b));
    }
}
