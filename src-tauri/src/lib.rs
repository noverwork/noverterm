mod db;
mod db_commands;
mod terminal;

use std::sync::Mutex;
use tauri::Manager;

// Re-export terminal commands
pub use terminal::{create_session, get_sessions, connect_session, disconnect_session, send_input, resize_pty, TerminalState};

// Re-export db commands
pub use db_commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            let pool = tauri::async_runtime::block_on(db::init_db())
                .expect("Failed to initialize database");

            app.manage(pool);
            Ok(())
        })
        .manage(Mutex::new(Vec::new()) as TerminalState)
        .invoke_handler(tauri::generate_handler![
            // Terminal commands
            greet,
            create_session,
            get_sessions,
            connect_session,
            disconnect_session,
            send_input,
            resize_pty,
            // Database commands - Groups
            db_commands::db_get_groups,
            db_commands::db_create_group,
            db_commands::db_update_group,
            db_commands::db_delete_group,
            // Database commands - SSH Keys
            db_commands::db_get_ssh_keys,
            db_commands::db_get_ssh_key,
            db_commands::db_create_ssh_key,
            db_commands::db_update_ssh_key,
            db_commands::db_delete_ssh_key,
            // Database commands - Sessions
            db_commands::db_get_all_sessions,
            db_commands::db_get_session,
            db_commands::db_create_session,
            db_commands::db_update_session,
            db_commands::db_delete_session,
            // Database commands - Port Forwards
            db_commands::db_get_port_forwards,
            db_commands::db_get_port_forward,
            db_commands::db_create_port_forward,
            db_commands::db_update_port_forward,
            db_commands::db_delete_port_forward,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
