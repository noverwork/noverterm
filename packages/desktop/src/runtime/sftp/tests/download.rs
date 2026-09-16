use super::*;

fn remote_data(size: usize) -> Vec<u8> {
    (0..size).map(|index| (index % 251) as u8).collect()
}

#[tokio::test]
async fn test_download_small_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("small-download.bin");
    let local_path = local_path.to_string_lossy().into_owned();
    let data = remote_data(1024);
    let session = SftpSession::mock_with_download_file("sftp-1", "/remote/small.bin", data.clone());
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

    let bytes = session
        .download(
            "/remote/small.bin",
            &local_path,
            "download-1".to_string(),
            TransferCancellation::new(),
            Some(progress_tx),
        )
        .await
        .expect("small file download should succeed");

    assert_eq!(bytes, 1024);
    let downloaded = tokio::fs::read(&local_path)
        .await
        .expect("downloaded file should be readable");
    assert_eq!(downloaded, data);

    let callback_events = session.mock_progress_events();
    assert!(!callback_events.is_empty());
    let final_event = callback_events
        .last()
        .expect("download should record a final progress event");
    assert_eq!(final_event.transfer_id, "download-1");
    assert_eq!(final_event.bytes_transferred, 1024);
    assert_eq!(final_event.total_bytes, 1024);
    assert_eq!(final_event.direction, TransferDirection::Download);

    let channel_event = progress_rx
        .try_recv()
        .expect("download should send progress through channel");
    assert_eq!(channel_event.bytes_transferred, 1024);
    assert_eq!(channel_event.direction, TransferDirection::Download);
}

#[tokio::test]
async fn test_download_large_file_streaming() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("large-download.bin");
    let local_path = local_path.to_string_lossy().into_owned();
    let file_size = 10 * 1024 * 1024;
    let data = remote_data(file_size);
    let session = SftpSession::mock_with_download_file("sftp-1", "/remote/large.bin", data.clone());

    let bytes = session
        .download(
            "/remote/large.bin",
            &local_path,
            "download-large".to_string(),
            TransferCancellation::new(),
            None,
        )
        .await
        .expect("large file download should succeed");

    assert_eq!(bytes, file_size as u64);
    let metadata = tokio::fs::metadata(&local_path)
        .await
        .expect("downloaded file should stat");
    assert_eq!(metadata.len(), file_size as u64);
    let callback_events = session.mock_progress_events();
    assert!(!callback_events.is_empty());
    let final_event = callback_events
        .last()
        .expect("download should record final progress");
    assert_eq!(final_event.bytes_transferred, file_size as u64);
    assert_eq!(final_event.direction, TransferDirection::Download);
}

#[tokio::test]
async fn test_download_cancel() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("cancel-download.bin");
    let local_path = local_path.to_string_lossy().into_owned();
    let data = remote_data(10 * 1024 * 1024);
    let session = SftpSession::mock_with_download_file("sftp-1", "/remote/cancel.bin", data);
    let cancel = TransferCancellation::new();
    let cancel_handle = cancel.clone();

    let download = session.download(
        "/remote/cancel.bin",
        &local_path,
        "download-cancel".to_string(),
        cancel,
        None,
    );
    let cancel_download = async {
        tokio::time::sleep(Duration::from_millis(5)).await;
        cancel_handle.cancel();
    };

    let (result, _) = tokio::join!(download, cancel_download);

    assert!(matches!(
        result,
        Err(SftpError::OperationFailed(error)) if error.contains("cancelled")
    ));
    assert!(!tokio::fs::try_exists(&local_path)
        .await
        .expect("local path existence should be checkable"));
}

#[tokio::test]
async fn test_download_error_cleanup() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("error-download.bin");
    let local_path = local_path.to_string_lossy().into_owned();
    let data = remote_data(1024 * 1024);
    let session = SftpSession::mock_with_download_error(
        "sftp-1",
        "/remote/error.bin",
        data,
        "remote read failed",
    );

    let result = session
        .download(
            "/remote/error.bin",
            &local_path,
            "download-error".to_string(),
            TransferCancellation::new(),
            None,
        )
        .await;

    assert_eq!(
        result,
        Err(SftpError::OperationFailed("remote read failed".to_string()))
    );
    assert!(!tokio::fs::try_exists(&local_path)
        .await
        .expect("local path existence should be checkable"));
}
