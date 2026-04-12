mod db;
mod models;
mod schema;

use crate::db::{init_pool, run_migrations, Pool};
use crate::models::{NewSetting, Setting, User};
use crate::schema::settings::dsl::settings;
use crate::schema::users::dsl::users;
use diesel::prelude::*;
use diesel::upsert::excluded;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_users(pool: State<'_, Pool>) -> Result<Vec<User>, String> {
    let mut connection = pool.get().map_err(|error| error.to_string())?;

    users
        .select(User::as_select())
        .load::<User>(&mut connection)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_setting(key: String, pool: State<'_, Pool>) -> Result<Option<Setting>, String> {
    let mut connection = pool.get().map_err(|error| error.to_string())?;

    settings
        .filter(schema::settings::key.eq(&key))
        .select(Setting::as_select())
        .first::<Setting>(&mut connection)
        .optional()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_setting(key: String, value: String, pool: State<'_, Pool>) -> Result<(), String> {
    let mut connection = pool.get().map_err(|error| error.to_string())?;

    diesel::insert_into(settings)
        .values(&NewSetting { key, value })
        .on_conflict(schema::settings::key)
        .do_update()
        .set(schema::settings::value.eq(excluded(schema::settings::value)))
        .execute(&mut connection)
        .map(|_| ())
        .map_err(|error| error.to_string())
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
            let app_handle = app.handle().clone();
            let db_path = resolve_db_path(&app_handle)?;
            let db_path = db_path.to_string_lossy().into_owned();
            let pool = init_pool(&db_path);

            run_migrations(&pool);
            app_handle.manage(pool);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_users,
            get_setting,
            set_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
