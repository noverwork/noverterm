mod terminal;

use std::sync::Mutex;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Re-export terminal commands
pub use terminal::{create_session, get_sessions, connect_session, disconnect_session, send_input, resize_pty, TerminalState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(Vec::new()) as TerminalState)
        .invoke_handler(tauri::generate_handler![
            greet,
            create_session,
            get_sessions,
            connect_session,
            disconnect_session,
            send_input,
            resize_pty
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
