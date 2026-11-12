//! Signs release files for the in-app updater: `update-signer FILE...` writes
//! `FILE.sig` next to each, in the format the old Tauri updater used, so both
//! the GPUI app and older Tauri builds can verify it.
//!
//! Reads the key from `TAURI_SIGNING_PRIVATE_KEY` and its password from
//! `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (empty when unset). The key is
//! encrypted with an empty password, which minisign before 0.8 cannot open.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("TAURI_SIGNING_PRIVATE_KEY")?;
    let password = std::env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD").unwrap_or_default();
    let secret = secret_key(&key, password)?;
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        return Err("usage: update-signer FILE...".into());
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    for file in files {
        let name = std::path::Path::new(&file)
            .file_name()
            .ok_or("not a file")?
            .to_string_lossy();
        let trusted = format!("timestamp:{timestamp}\tfile:{name}");
        let data = std::fs::File::open(&file)?;
        let signature = minisign::sign(
            None,
            &secret,
            data,
            Some(&trusted),
            Some("signature from tauri secret key"),
        )?;
        std::fs::write(
            format!("{file}.sig"),
            STANDARD.encode(signature.to_string()),
        )?;
        println!("signed {file}");
    }
    Ok(())
}

fn secret_key(
    base64_key: &str,
    password: String,
) -> Result<minisign::SecretKey, Box<dyn std::error::Error>> {
    let key = String::from_utf8(STANDARD.decode(base64_key.trim())?)?;
    Ok(minisign::SecretKeyBox::from_string(&key)?.into_secret_key(Some(password))?)
}

#[cfg(test)]
mod tests {
    // Tauri's test key, encrypted with an empty password like the release key.
    const EMPTY_PASSWORD_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5dkpDN09RZm5GeVAzc2RuYlNzWVVJelJRQnNIV2JUcGVXZUplWXZXYXpqUUFBQkFBQUFBQUFBQUFBQUlBQUFBQTZrN2RnWGh5dURxSzZiL1ZQSDdNcktiaHRxczQwMXdQelRHbjRNcGVlY1BLMTBxR2dpa3I3dDE1UTVDRDE4MXR4WlQwa1BQaXdxKy9UU2J2QmVSNXhOQWFDeG1GSVllbUNpTGJQRkhhTnROR3I5RmdUZi90OGtvaGhJS1ZTcjdZU0NyYzhQWlQ5cGM9Cg==";

    #[test]
    fn opens_a_key_encrypted_with_an_empty_password() {
        assert!(super::secret_key(EMPTY_PASSWORD_KEY, String::new()).is_ok());
        assert!(super::secret_key(EMPTY_PASSWORD_KEY, "wrong".into()).is_err());
    }
}
