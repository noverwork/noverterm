use russh::keys::ssh_key::{Algorithm, HashAlg};

pub(crate) fn load_key_pair(
    private_key: &str,
    passphrase: Option<&str>,
) -> Result<russh::keys::PrivateKey, String> {
    russh::keys::decode_secret_key(private_key, passphrase).map_err(|error| match error {
        russh::keys::Error::KeyIsEncrypted => {
            "SSH key requires a passphrase, but none was provided".to_string()
        }
        _ => format!("Failed to parse SSH key: {error}"),
    })
}

pub(crate) fn rsa_hash_candidates(server_best: Option<Option<HashAlg>>) -> Vec<Option<HashAlg>> {
    match server_best {
        Some(hash_alg) => vec![hash_alg],
        None => vec![Some(HashAlg::Sha512), Some(HashAlg::Sha256), None],
    }
}

pub(crate) fn is_rsa_key(key: &russh::keys::PrivateKey) -> bool {
    matches!(key.algorithm(), Algorithm::Rsa { .. })
}

#[cfg(test)]
mod tests {
    use rsa::{
        pkcs1::{EncodeRsaPrivateKey, LineEnding},
        rand_core::{CryptoRng, RngCore},
    };
    use russh::keys::ssh_key::{Algorithm, HashAlg};

    use super::{is_rsa_key, load_key_pair, rsa_hash_candidates};

    struct TestRng(u64);

    impl RngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }

        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn fill_bytes(&mut self, dst: &mut [u8]) {
            for chunk in dst.chunks_mut(8) {
                let bytes = self.next_u64().to_ne_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
        }
    }

    impl CryptoRng for TestRng {}

    #[test]
    fn loads_legacy_rsa_pkcs1_private_key() {
        let mut rng = TestRng(0x4e6f_7665_7254_6572);
        let rsa_key =
            rsa::RsaPrivateKey::new(&mut rng, 2048).expect("test RSA key should be generated");
        let pem = rsa_key
            .to_pkcs1_pem(LineEnding::LF)
            .expect("test RSA key should encode as PKCS#1 PEM");

        let key = load_key_pair(pem.as_str(), None).expect("legacy RSA key should parse");

        assert!(matches!(key.algorithm(), Algorithm::Rsa { .. }));
        assert!(is_rsa_key(&key));
    }

    #[test]
    fn tries_rsa_sha2_before_legacy_when_server_does_not_advertise_sig_algs() {
        let mut rng = TestRng(0x4e6f_7665_7254_6572);
        let rsa_key =
            rsa::RsaPrivateKey::new(&mut rng, 2048).expect("test RSA key should be generated");
        let pem = rsa_key
            .to_pkcs1_pem(LineEnding::LF)
            .expect("test RSA key should encode as PKCS#1 PEM");
        let key = load_key_pair(pem.as_str(), None).expect("legacy RSA key should parse");
        assert!(is_rsa_key(&key));

        assert_eq!(
            rsa_hash_candidates(None),
            vec![Some(HashAlg::Sha512), Some(HashAlg::Sha256), None]
        );
    }

    #[test]
    fn uses_advertised_rsa_hash_when_server_reports_sig_algs() {
        let mut rng = TestRng(0x4e6f_7665_7254_6572);
        let rsa_key =
            rsa::RsaPrivateKey::new(&mut rng, 2048).expect("test RSA key should be generated");
        let pem = rsa_key
            .to_pkcs1_pem(LineEnding::LF)
            .expect("test RSA key should encode as PKCS#1 PEM");
        let key = load_key_pair(pem.as_str(), None).expect("legacy RSA key should parse");
        assert!(is_rsa_key(&key));

        assert_eq!(
            rsa_hash_candidates(Some(Some(HashAlg::Sha256))),
            vec![Some(HashAlg::Sha256)]
        );
        assert_eq!(rsa_hash_candidates(Some(None)), vec![None]);
    }
}
