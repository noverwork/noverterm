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
        crate::auth::auth_login,
        crate::auth::auth_logout,
        bootstrap_restore,
        bootstrap_load_metadata,
        bootstrap_save_connection,
        bootstrap_delete_connection,
        bootstrap_save_setting,
        crate::settings::get_setting,
        crate::settings::set_setting,
        crate::settings::get_all_settings,
        crate::connect::ssh_connect,
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
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/bindings.ts");

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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let specta_builder = command_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let settings_path = app_data_dir.join("settings.json");
            let settings = SettingsManager::new(settings_path);
            app.manage(settings);

            let backend_url = std::env::var("NOVERTERM_BACKEND_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
            let auth_manager = DesktopAuthManager::from_backend_url(backend_url);
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

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet_includes_name() {
        assert!(greet("Noverterm").contains("Noverterm"));
    }
}
