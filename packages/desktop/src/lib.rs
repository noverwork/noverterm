pub mod bootstrap;
pub mod connect;
pub mod runtime;
pub mod sftp;
pub mod store;
pub mod trust;

pub use bootstrap::{export_types, run};

pub const FEATURE_BOUNDARIES: &[&str] = &[
    "bootstrap",
    "store",
    "runtime/ssh",
    "runtime/local",
    "trust",
    "connect",
    "sftp",
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
                "bootstrap",
                "store",
                "runtime/ssh",
                "runtime/local",
                "trust",
                "connect",
                "sftp",
            ]
        );
    }
}
