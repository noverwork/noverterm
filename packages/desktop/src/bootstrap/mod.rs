use std::env;
use std::fs;
use std::path::PathBuf;

use specta_typescript::Typescript;
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};
use tracing_subscriber::EnvFilter;

use crate::auth::DesktopAuthManager;
use crate::runtime::local::LocalSessionManager;
use crate::runtime::ssh::SshSessionManager;
use crate::settings::SettingsManager;
use crate::trust::SshTrustStore;

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_restore(
    auth_manager: tauri::State<'_, DesktopAuthManager>,
) -> Result<Option<crate::auth::AuthBootstrapStatus>, String> {
    auth_manager.restore().await
}

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_load_metadata(
    auth_manager: tauri::State<'_, DesktopAuthManager>,
) -> Result<crate::auth::BootstrapMetadata, String> {
    auth_manager.load_bootstrap_metadata().await
}

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_save_connection(
    connection: crate::auth::SaveConnectionInput,
    auth_manager: tauri::State<'_, DesktopAuthManager>,
) -> Result<shared::SshHostRecord, String> {
    auth_manager.save_connection(connection).await
}

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_delete_connection(
    id: String,
    ssh_key_id: Option<String>,
    auth_manager: tauri::State<'_, DesktopAuthManager>,
) -> Result<(), String> {
    auth_manager.delete_connection(id).await?;

    if let Some(ssh_key_id) = ssh_key_id {
        auth_manager.delete_key(ssh_key_id).await?;
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn bootstrap_save_setting(
    setting: shared::Setting,
    auth_manager: tauri::State<'_, DesktopAuthManager>,
) -> Result<shared::Setting, String> {
    auth_manager.upsert_setting(setting).await
}

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn command_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        greet,
        crate::connect::ssh_connect_direct,
        crate::connect::ssh_confirm_host_trust,
        crate::connect::ssh_write,
        crate::connect::ssh_resize,
        crate::connect::ssh_disconnect,
        crate::connect::local_connect,
        crate::connect::local_write,
        crate::connect::local_resize,
        crate::connect::local_disconnect
    ])
}

pub fn export_types() -> Result<(), Box<dyn std::error::Error>> {
    let builder = command_builder();
    let bindings_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/src/bindings.ts");

    builder.export(Typescript::default(), &bindings_path)?;

    let bindings = std::fs::read_to_string(&bindings_path)?;
    let globals_marker = "/** tauri-specta globals **/";
    let prefix = bindings
        .split(globals_marker)
        .next()
        .ok_or("failed to locate tauri-specta globals section")?
        .replace("error: e  as any", "error: String(e)");
    let sanitized_bindings = format!(
        "{prefix}{globals_marker}\n\nimport {{ invoke as TAURI_INVOKE }} from \"@tauri-apps/api/core\";\n\nexport type Result<T, E> =\n\t| {{ status: \"ok\"; data: T }}\n\t| {{ status: \"error\"; error: E }};\n"
    );

    std::fs::write(bindings_path, sanitized_bindings)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    load_runtime_env();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let specta_builder = command_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let settings_path = app_data_dir.join("settings.json");
            let settings = SettingsManager::new(settings_path);
            app.manage(settings);

            let backend_url = backend_base_url();
            let auth_tokens_path = app_data_dir.join("auth").join("tokens.json");
            let auth_manager = DesktopAuthManager::from_backend_url(backend_url, auth_tokens_path);
            app.manage(auth_manager);

            let trust_path = app_data_dir.join("trust").join("ssh_hosts.json");
            let trust_store = SshTrustStore::new(trust_path)?;
            app.manage(trust_store);

            let ssh_manager = SshSessionManager::new();
            app.manage(ssh_manager);

            let local_manager = LocalSessionManager::new();
            app.manage(local_manager);

            Ok(())
        })
        .invoke_handler(specta_builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn load_runtime_env() {
    for path in env_candidates() {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };

        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };

            if env::var_os(key.trim()).is_none() {
                env::set_var(key.trim(), value.trim());
            }
        }
    }
}

fn env_candidates() -> [PathBuf; 2] {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [manifest_dir.join(".env"), manifest_dir.join("../backend/.env")]
}

fn backend_base_url() -> String {
    if let Ok(url) = env::var("NOVERTERM_BACKEND_URL") {
        return url;
    }

    if let Ok(url) = env::var("BACKEND_API_URL") {
        return url;
    }

    let host = env::var("BACKEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("BACKEND_PORT").unwrap_or_else(|_| "3000".to_string());
    format!("http://{host}:{port}")
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::{Mutex, OnceLock};

    use super::{backend_base_url, greet};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn greet_includes_name() {
        assert!(greet("Noverterm").contains("Noverterm"));
    }

    #[test]
    fn backend_base_url_prefers_composed_host_and_port() {
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock should be acquirable");

        let original_backend_url = env::var_os("NOVERTERM_BACKEND_URL");
        let original_backend_api_url = env::var_os("BACKEND_API_URL");
        let original_host = env::var_os("BACKEND_HOST");
        let original_port = env::var_os("BACKEND_PORT");

        env::remove_var("NOVERTERM_BACKEND_URL");
        env::remove_var("BACKEND_API_URL");
        env::set_var("BACKEND_HOST", "127.0.0.1");
        env::set_var("BACKEND_PORT", "4310");

        assert_eq!(backend_base_url(), "http://127.0.0.1:4310");

        match original_backend_url {
            Some(value) => env::set_var("NOVERTERM_BACKEND_URL", value),
            None => env::remove_var("NOVERTERM_BACKEND_URL"),
        }
        match original_backend_api_url {
            Some(value) => env::set_var("BACKEND_API_URL", value),
            None => env::remove_var("BACKEND_API_URL"),
        }
        match original_host {
            Some(value) => env::set_var("BACKEND_HOST", value),
            None => env::remove_var("BACKEND_HOST"),
        }
        match original_port {
            Some(value) => env::set_var("BACKEND_PORT", value),
            None => env::remove_var("BACKEND_PORT"),
        }
    }
}
