pub mod auth;
pub mod bootstrap;
pub mod config;
pub mod db;
pub mod email;
pub mod host_groups;
pub mod hosts;
pub mod keys;
pub mod settings;

pub use bootstrap::run;

pub const FEATURE_BOUNDARIES: &[&str] = &["auth", "host_groups", "hosts", "keys", "settings"];

pub fn feature_boundaries() -> &'static [&'static str] {
    FEATURE_BOUNDARIES
}

pub async fn healthcheck() -> &'static str {
    "Backend running"
}

#[cfg(test)]
mod tests {
    use super::feature_boundaries;

    #[test]
    fn backend_feature_boundaries_match_control_plane_ownership() {
        assert_eq!(
            feature_boundaries(),
            &["auth", "host_groups", "hosts", "keys", "settings"]
        );
    }
}

#[cfg(test)]
mod test_support;
