//! Signs release files for the in-app updater: `update-signer FILE...` writes
//! `FILE.sig` next to each, in the format the old Tauri updater used, so both
//! the GPUI app and older Tauri builds can verify it.
//!
//! Reads the key from `TAURI_SIGNING_PRIVATE_KEY` and its password from
//! `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (empty when unset). The Tauri CLI's
//! own `signer sign` rejects an empty password, which this key uses.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("TAURI_SIGNING_PRIVATE_KEY")?;
    let password = std::env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD").unwrap_or_default();
    let key = String::from_utf8(STANDARD.decode(key.trim())?)?;
    let secret = minisign::SecretKeyBox::from_string(&key)?.into_secret_key(Some(password))?;
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
