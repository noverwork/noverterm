mod config;

use crate::config::SettingsManager;
use shared::Setting;
use specta_typescript::Typescript;
use tauri::{Manager, State};
use tauri_specta::{collect_commands, Builder};
use tracing_subscriber::EnvFilter;

#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
#[specta::specta]
fn get_setting(key: String, settings: State<'_, SettingsManager>) -> Option<Setting> {
    settings.get(&key)
}

#[tauri::command]
#[specta::specta]
fn set_setting(setting: Setting, settings: State<'_, SettingsManager>) -> Result<(), String> {
    settings.set(setting)
}

#[tauri::command]
#[specta::specta]
fn get_all_settings(settings: State<'_, SettingsManager>) -> Vec<Setting> {
    settings.all()
}

fn command_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        greet,
        get_setting,
        set_setting,
        get_all_settings
    ])
}

pub fn export_types() -> Result<(), Box<dyn std::error::Error>> {
    let builder = command_builder();
    let bindings_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/bindings.ts");

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

            Ok(())
        })
        .invoke_handler(specta_builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
