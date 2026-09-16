use super::*;

#[tokio::test]
async fn test_upload_small_file() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("small.bin");
    write_temp_file(&local_path, 1024).await;
    let local_path = local_path.to_string_lossy().into_owned();
    let session = SftpSession::mock("sftp-1");
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();

    let bytes = session
        .upload(
            &local_path,
            "/remote/small.bin",
            "transfer-1".to_string(),
            TransferCancellation::new(),
            Some(progress_tx),
        )
        .await
        .expect("small file upload should succeed");

    assert_eq!(bytes, 1024);
    let entry = session
        .stat("/remote/small.bin")
        .await
        .expect("uploaded mock file should stat");
    assert_eq!(entry.size, 1024);

    let callback_events = session.mock_progress_events();
    assert!(!callback_events.is_empty());
    let final_event = callback_events
        .last()
        .expect("upload should record a final progress event");
    assert_eq!(final_event.transfer_id, "transfer-1");
    assert_eq!(final_event.bytes_transferred, 1024);
    assert_eq!(final_event.total_bytes, 1024);
    assert_eq!(final_event.direction, TransferDirection::Upload);

    let channel_event = progress_rx
        .try_recv()
        .expect("upload should send progress through channel");
    assert_eq!(channel_event.bytes_transferred, 1024);
}

#[tokio::test]
async fn test_upload_large_file_streaming() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("large.bin");
    let file_size = 10 * 1024 * 1024;
    write_temp_file(&local_path, file_size).await;
    let local_path = local_path.to_string_lossy().into_owned();
    let session = SftpSession::mock("sftp-1");

    let bytes = session
        .upload(
            &local_path,
            "/remote/large.bin",
            "transfer-large".to_string(),
            TransferCancellation::new(),
            None,
        )
        .await
        .expect("large file upload should succeed");

    assert_eq!(bytes, file_size as u64);
    let entry = session
        .stat("/remote/large.bin")
        .await
        .expect("uploaded mock file should stat");
    assert_eq!(entry.size, file_size as u64);
    assert_eq!(entry.file_type, FileType::File);
}

#[tokio::test]
async fn test_upload_cancel() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("cancel.bin");
    write_temp_file(&local_path, 10 * 1024 * 1024).await;
    let local_path = local_path.to_string_lossy().into_owned();
    let session = SftpSession::mock("sftp-1");
    let cancel = TransferCancellation::new();
    let cancel_handle = cancel.clone();

    let upload = session.upload(
        &local_path,
        "/remote/cancel.bin",
        "transfer-cancel".to_string(),
        cancel,
        None,
    );
    let cancel_upload = async {
        tokio::time::sleep(Duration::from_millis(5)).await;
        cancel_handle.cancel();
    };

    let (result, _) = tokio::join!(upload, cancel_upload);

    assert!(matches!(
        result,
        Err(SftpError::OperationFailed(error)) if error.contains("cancelled")
    ));
    assert!(!session.mock_contains_path("/remote/cancel.bin"));
}

#[tokio::test]
async fn test_upload_directory_tree() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let nested = temp_dir.path().join("tree/nested");
    tokio::fs::create_dir_all(&nested)
        .await
        .expect("nested dir should be created");
    write_temp_file(&temp_dir.path().join("tree/root.bin"), 1024).await;
    write_temp_file(&nested.join("leaf.bin"), 2048).await;
    let local_root = temp_dir.path().join("tree").to_string_lossy().into_owned();
    let session = SftpSession::mock("sftp-1");

    let total = session
        .upload(
            &local_root,
            "/remote/tree",
            "transfer-dir".to_string(),
            TransferCancellation::new(),
            None,
        )
        .await
        .expect("directory upload should succeed");

    assert_eq!(total, 1024 + 2048);
    assert!(session.mock_contains_path("/remote/tree"));
    assert!(session.mock_contains_path("/remote/tree/root.bin"));
    assert!(session.mock_contains_path("/remote/tree/nested"));
    assert!(session.mock_contains_path("/remote/tree/nested/leaf.bin"));
}

#[tokio::test]
async fn test_upload_directory_tree_over_existing_remote_dir() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    tokio::fs::create_dir_all(temp_dir.path().join("tree"))
        .await
        .expect("dir should be created");
    write_temp_file(&temp_dir.path().join("tree/root.bin"), 1024).await;
    let local_root = temp_dir.path().join("tree").to_string_lossy().into_owned();
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/remote/tree".to_string(), MockEntry::directory(None))]),
    );

    let total = session
        .upload(
            &local_root,
            "/remote/tree",
            "transfer-dir".to_string(),
            TransferCancellation::new(),
            None,
        )
        .await
        .expect("upload into an existing remote directory should succeed");

    assert_eq!(total, 1024);
    assert!(session.mock_contains_path("/remote/tree/root.bin"));
}

#[tokio::test]
async fn test_upload_error_cleanup() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_path = temp_dir.path().join("error.bin");
    write_temp_file(&local_path, 1024 * 1024).await;
    let local_path = local_path.to_string_lossy().into_owned();
    let session =
        SftpSession::mock_with_upload_result("sftp-1", Err("remote write failed".to_string()));

    let result = session
        .upload(
            &local_path,
            "/remote/error.bin",
            "transfer-error".to_string(),
            TransferCancellation::new(),
            None,
        )
        .await;

    assert_eq!(
        result,
        Err(SftpError::OperationFailed(
            "remote write failed".to_string()
        ))
    );
    assert!(!session.mock_contains_path("/remote/error.bin"));
}
