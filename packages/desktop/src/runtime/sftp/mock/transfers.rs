use std::collections::HashMap;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::super::transfer::{ProgressEmitter, UPLOAD_CHUNK_SIZE};
use super::super::{SftpError, TransferCancellation, TransferDirection, TransferProgress};
use super::{MockEntry, MockFileType};

#[allow(clippy::too_many_arguments)]
pub(in super::super) async fn upload_mock(
    entries: &std::sync::Mutex<HashMap<String, MockEntry>>,
    upload_result: &Result<u64, String>,
    progress_callbacks: &std::sync::Mutex<Vec<TransferProgress>>,
    local_path: &str,
    remote_path: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let mut local_file = tokio::fs::File::open(local_path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
    let total_bytes = local_file
        .metadata()
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?
        .len();
    let mut progress = ProgressEmitter::new(
        transfer_id,
        total_bytes,
        TransferDirection::Upload,
        progress_tx,
    );
    let mut buffer = vec![0; UPLOAD_CHUNK_SIZE];
    let mut bytes_transferred = 0;
    let mut injected_error = upload_result.as_ref().err().cloned();

    let result = async {
        loop {
            if cancel.is_cancelled() {
                return Err(SftpError::OperationFailed("upload cancelled".to_string()));
            }

            let read_count = local_file
                .read(&mut buffer)
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            if read_count == 0 {
                break;
            }

            bytes_transferred += read_count as u64;
            {
                let mut entries = entries
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
                entries.insert(
                    remote_path.to_string(),
                    MockEntry::file(bytes_transferred, None),
                );
            }

            if let Some(error) = injected_error.take() {
                return Err(SftpError::OperationFailed(error));
            }

            if let Some(event) = progress.maybe_emit(bytes_transferred) {
                progress_callbacks
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?
                    .push(event);
            }

            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        if let Some(event) = progress.emit_final(bytes_transferred) {
            progress_callbacks
                .lock()
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?
                .push(event);
        }

        Ok(bytes_transferred)
    }
    .await;

    if result.is_err() {
        if let Ok(mut entries) = entries.lock() {
            entries.remove(remote_path);
        }
    }

    result
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) async fn download_mock(
    entries: &std::sync::Mutex<HashMap<String, MockEntry>>,
    remote_files: &std::sync::Mutex<HashMap<String, Vec<u8>>>,
    download_result: &Result<u64, String>,
    progress_callbacks: &std::sync::Mutex<Vec<TransferProgress>>,
    remote_path: &str,
    local_path: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let data = {
        let entries = entries
            .lock()
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        let entry = entries
            .get(remote_path)
            .copied()
            .ok_or_else(|| SftpError::NotFound(remote_path.to_string()))?;

        if entry.file_type != MockFileType::File {
            return Err(SftpError::IsADirectory);
        }

        let remote_files = remote_files
            .lock()
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        match remote_files.get(remote_path) {
            Some(data) => data.clone(),
            None => vec![
                0;
                usize::try_from(entry.size)
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?
            ],
        }
    };

    let total_bytes = data.len() as u64;
    let mut local_file = tokio::fs::File::create(local_path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
    let mut progress = ProgressEmitter::new(
        transfer_id,
        total_bytes,
        TransferDirection::Download,
        progress_tx,
    );
    let mut bytes_transferred = 0;
    let mut injected_error = download_result.as_ref().err().cloned();

    let result = async {
        for chunk in data.chunks(UPLOAD_CHUNK_SIZE) {
            if cancel.is_cancelled() {
                return Err(SftpError::OperationFailed("download cancelled".to_string()));
            }

            local_file
                .write_all(chunk)
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            bytes_transferred += chunk.len() as u64;

            if let Some(error) = injected_error.take() {
                return Err(SftpError::OperationFailed(error));
            }

            if let Some(event) = progress.maybe_emit(bytes_transferred) {
                progress_callbacks
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?
                    .push(event);
            }

            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        local_file
            .flush()
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        if let Some(event) = progress.emit_final(bytes_transferred) {
            progress_callbacks
                .lock()
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?
                .push(event);
        }

        Ok(bytes_transferred)
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(local_path).await;
    }

    result
}
