use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri::Manager;
use tauri_specta::{collect_commands, Builder};
use tracing_subscriber::EnvFilter;

use crate::runtime::local::LocalSessionManager;
use crate::runtime::port_forward::PortForwardManager;
use crate::runtime::ssh::SshSessionManager;
use crate::sftp::state::TransferState;
use crate::trust::SshTrustStore;

#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn command_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        greet,
        crate::store::db_backup,
        crate::store::hosts::host_list,
        crate::store::hosts::host_save,
        crate::store::hosts::host_delete,
        crate::store::groups::host_group_list,
        crate::store::groups::host_group_create,
        crate::store::groups::host_group_delete,
        crate::store::keys::key_list,
        crate::store::keys::key_create,
        crate::store::keys::key_update,
        crate::store::keys::key_delete,
        crate::store::keys::key_secret,
        crate::store::settings::get_all_settings,
        crate::store::settings::get_setting,
        crate::store::settings::set_setting,
        crate::store::snippets::snippet_list,
        crate::store::snippets::snippet_get,
        crate::store::snippets::snippet_create,
        crate::store::snippets::snippet_update,
        crate::store::snippets::snippet_delete,
        crate::connect::ssh_connect_direct,
        crate::connect::ssh_confirm_host_trust,
        crate::connect::ssh_probe_host_info,
        crate::connect::ssh_write,
        crate::connect::ssh_resize,
        crate::connect::ssh_disconnect,
        crate::connect::ssh_start_local_port_forward,
        crate::connect::ssh_stop_port_forward,
        crate::connect::local_connect,
        crate::connect::local_write,
        crate::connect::local_resize,
        crate::connect::local_disconnect,
        crate::connect::port_forward_start,
        crate::connect::port_forward_stop,
        crate::connect::port_forward_list,
        crate::sftp::sftp_open,
        crate::sftp::sftp_close,
        crate::sftp::sftp_connect_direct,
        crate::sftp::sftp_home_dir,
        crate::sftp::sftp_list_dir,
        crate::sftp::sftp_stat,
        crate::sftp::sftp_mkdir,
        crate::sftp::sftp_remove,
        crate::sftp::sftp_rename,
        crate::sftp::sftp_upload,
        crate::sftp::sftp_download,
        crate::sftp::sftp_cancel_transfer,
        crate::sftp::local_list_dir,
        crate::sftp::local_stat,
        crate::sftp::local_mkdir,
        crate::sftp::local_remove,
        crate::sftp::local_rename,
        crate::trust::known_hosts_get,
        crate::trust::known_hosts_remove
    ])
}

pub fn export_types() -> Result<(), Box<dyn std::error::Error>> {
    let builder = command_builder();
    let bindings_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/src/bindings.ts");

    builder.export(
        Typescript::default().bigint(BigIntExportBehavior::Number),
        &bindings_path,
    )?;

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            // Debug builds get their own database so `cargo make dev` cannot
            // scribble over the installed app's connections and host keys.
            let database_name = if cfg!(debug_assertions) {
                "noverterm-dev.db"
            } else {
                "noverterm.db"
            };
            let pool = crate::store::init_pool(&app_data_dir.join(database_name))?;
            app.manage(SshTrustStore::new(pool.clone()));
            app.manage(pool);

            let ssh_manager = SshSessionManager::new();
            app.manage(ssh_manager);

            let local_manager = LocalSessionManager::new();
            app.manage(local_manager);

            let pf_manager = PortForwardManager::new();
            app.manage(pf_manager);

            let transfer_state = TransferState::new();
            app.manage(transfer_state);

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

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
