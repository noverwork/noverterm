use std::path::Path;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_path = Path::new(manifest_dir).join(".env");

    if env_path.exists() {
        let content = std::fs::read_to_string(&env_path).expect("Failed to read .env");
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                println!("cargo:rustc-env={key}={value}");
            }
        }
    }

    tauri_build::build()
}
