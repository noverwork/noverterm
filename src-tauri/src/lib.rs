mod db;
mod models;
mod schema;

use crate::db::{init_pool, run_migrations, Pool};
use crate::models::User;
use crate::schema::users::dsl::users;
use diesel::prelude::*;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_users(pool: State<'_, Pool>) -> Result<Vec<User>, String> {
    let mut connection = pool.get().map_err(|error| error.to_string())?;

    users
        .load::<User>(&mut connection)
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
        .invoke_handler(tauri::generate_handler![greet, get_users])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
