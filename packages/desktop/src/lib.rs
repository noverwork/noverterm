pub mod auth;
pub mod bootstrap;
pub mod connect;
pub mod runtime;
pub mod settings;
pub mod trust;

pub use bootstrap::{export_types, run};

pub const FEATURE_BOUNDARIES: &[&str] = &[
    "auth",
    "bootstrap",
    "runtime/ssh",
    "runtime/local",
    "trust",
    "settings",
    "connect",
];

pub fn feature_boundaries() -> &'static [&'static str] {
    FEATURE_BOUNDARIES
}

#[cfg(test)]
mod tests {
    use super::feature_boundaries;

    #[test]
    fn desktop_feature_boundaries_keep_runtime_and_trust_local() {
        assert_eq!(
            feature_boundaries(),
            &[
                "auth",
                "bootstrap",
                "runtime/ssh",
                "runtime/local",
                "trust",
                "settings",
                "connect",
            ]
        );
    }
}
