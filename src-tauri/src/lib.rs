mod config;
mod db;
mod models;
mod schema;

use crate::config::SettingsManager;
use crate::db::{init_pool, run_migrations};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_setting(key: String, settings: State<'_, SettingsManager>) -> Option<String> {
    settings.get(&key)
}

#[tauri::command]
fn set_setting(
    key: String,
    value: String,
    settings: State<'_, SettingsManager>,
) -> Result<(), String> {
    settings.set(key, value)
}

#[tauri::command]
fn get_all_settings(
    settings: State<'_, SettingsManager>,
) -> std::collections::HashMap<String, String> {
    settings.all()
}

fn resolve_db_path(app: &AppHandle) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    Ok(app_data_dir.join("novercpa.db"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let settings_path = app_data_dir.join("settings.json");
            let settings = SettingsManager::new(settings_path);
            app.manage(settings);

            let db_path = resolve_db_path(app.handle())?
                .to_string_lossy()
                .into_owned();
            let pool = init_pool(&db_path);
            run_migrations(&pool);
            app.manage(pool);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_setting,
            set_setting,
            get_all_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
