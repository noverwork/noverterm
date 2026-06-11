pub mod state;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::runtime::local_fs;
use crate::runtime::sftp::{
    FileEntry, TransferCancellation, TransferComplete, TransferDirection, TransferError,
};
use crate::runtime::ssh::SshSessionManager;
use crate::trust::SshTrustStore;

use self::state::TransferState;

#[tauri::command]
#[specta::specta]
pub async fn sftp_open(
    session_id: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<String, String> {
    ssh_manager.open_sftp(&session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_close(
    session_id: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.close_sftp(&session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_connect_direct(
    app: AppHandle,
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    private_key: Option<String>,
    passphrase: Option<String>,
    ssh_manager: State<'_, SshSessionManager>,
    trust_store: State<'_, SshTrustStore>,
) -> Result<String, String> {
    ssh_manager
        .connect_direct_sftp(
            app,
            &host,
            port,
            &username,
            password.as_deref(),
            private_key.as_deref(),
            passphrase.as_deref(),
            trust_store.inner().clone(),
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_list_dir(
    session_id: String,
    path: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<Vec<FileEntry>, String> {
    ssh_manager.sftp_list_dir(&session_id, &path).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_home_dir(
    session_id: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<String, String> {
    ssh_manager.sftp_home_dir(&session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_stat(
    session_id: String,
    path: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<FileEntry, String> {
    ssh_manager.sftp_stat(&session_id, &path).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_mkdir(
    session_id: String,
    path: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.sftp_mkdir(&session_id, &path).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_remove(
    session_id: String,
    path: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager.sftp_remove(&session_id, &path).await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_rename(
    session_id: String,
    old_path: String,
    new_path: String,
    ssh_manager: State<'_, SshSessionManager>,
) -> Result<(), String> {
    ssh_manager
        .sftp_rename(&session_id, &old_path, &new_path)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_upload(
    app: AppHandle,
    session_id: String,
    local_path: String,
    remote_path: String,
    ssh_manager: State<'_, SshSessionManager>,
    transfer_state: State<'_, TransferState>,
) -> Result<String, String> {
    spawn_transfer(
        app,
        session_id,
        local_path,
        remote_path,
        TransferDirection::Upload,
        ssh_manager.inner().clone(),
        transfer_state.inner(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_download(
    app: AppHandle,
    session_id: String,
    remote_path: String,
    local_path: String,
    ssh_manager: State<'_, SshSessionManager>,
    transfer_state: State<'_, TransferState>,
) -> Result<String, String> {
    spawn_transfer(
        app,
        session_id,
        remote_path,
        local_path,
        TransferDirection::Download,
        ssh_manager.inner().clone(),
        transfer_state.inner(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn sftp_cancel_transfer(
    transfer_id: String,
    transfer_state: State<'_, TransferState>,
) -> Result<(), String> {
    let cancellations = transfer_state.cancellations.lock().await;
    let cancellation = cancellations
        .get(&transfer_id)
        .ok_or_else(|| format!("Transfer not found: {transfer_id}"))?;
    cancellation.cancel();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn local_list_dir(path: String) -> Result<Vec<FileEntry>, String> {
    local_fs::local_list_dir(&path).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_stat(path: String) -> Result<FileEntry, String> {
    local_fs::local_stat(&path).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mkdir(path: String) -> Result<(), String> {
    local_fs::local_mkdir(&path).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_remove(path: String) -> Result<(), String> {
    local_fs::local_remove(&path).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_rename(old_path: String, new_path: String) -> Result<(), String> {
    local_fs::local_rename(&old_path, &new_path).await
}

async fn spawn_transfer(
    app: AppHandle,
    session_id: String,
    source_path: String,
    target_path: String,
    direction: TransferDirection,
    ssh_manager: SshSessionManager,
    transfer_state: &TransferState,
) -> Result<String, String> {
    let transfer_id = Uuid::new_v4().to_string();
    let cancellation = TransferCancellation::new();
    transfer_state
        .cancellations
        .lock()
        .await
        .insert(transfer_id.clone(), cancellation.clone());

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let progress_app = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = progress_app.emit("sftp://progress", progress);
        }
    });

    let task_transfer_id = transfer_id.clone();
    tokio::spawn(async move {
        let result = match direction {
            TransferDirection::Upload => {
                ssh_manager
                    .sftp_upload(
                        &session_id,
                        &source_path,
                        &target_path,
                        task_transfer_id.clone(),
                        cancellation,
                        Some(progress_tx),
                    )
                    .await
            }
            TransferDirection::Download => {
                ssh_manager
                    .sftp_download(
                        &session_id,
                        &source_path,
                        &target_path,
                        task_transfer_id.clone(),
                        cancellation,
                        Some(progress_tx),
                    )
                    .await
            }
        };

        match result {
            Ok(total_bytes) => {
                let _ = app.emit(
                    "sftp://complete",
                    TransferComplete {
                        transfer_id: task_transfer_id,
                        total_bytes,
                        direction,
                    },
                );
            }
            Err(error) => {
                let _ = app.emit(
                    "sftp://error",
                    TransferError {
                        transfer_id: task_transfer_id,
                        error,
                        direction,
                    },
                );
            }
        }
    });

    Ok(transfer_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sftp_commands_compile() {
        let _ = sftp_open;
        let _ = sftp_close;
        let _ = sftp_connect_direct;
        let _ = sftp_home_dir;
        let _ = sftp_list_dir;
        let _ = sftp_stat;
        let _ = sftp_mkdir;
        let _ = sftp_remove;
        let _ = sftp_rename;
        let _ = sftp_upload;
        let _ = sftp_download;
        let _ = sftp_cancel_transfer;
    }

    #[tokio::test]
    async fn test_local_list_dir_command() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        tokio::fs::write(temp_dir.path().join("file.txt"), "content")
            .await
            .expect("file should be written");

        let entries = local_list_dir(temp_dir.path().to_string_lossy().into_owned())
            .await
            .expect("local list command should succeed");

        assert!(entries.iter().any(|entry| entry.name == "file.txt"));
    }

    #[tokio::test]
    async fn test_local_mkdir_command() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let path = temp_dir.path().join("created");

        local_mkdir(path.to_string_lossy().into_owned())
            .await
            .expect("local mkdir command should succeed");

        assert!(path.is_dir());
    }
}
