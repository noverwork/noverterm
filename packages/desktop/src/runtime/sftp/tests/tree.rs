use super::*;

#[tokio::test]
async fn test_transfer_conflicts_lists_files_present_on_both_sides() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_root = temp_dir.path().join("tree");
    tokio::fs::create_dir_all(local_root.join("nested"))
        .await
        .expect("nested dir should be created");
    write_temp_file(&local_root.join("root.bin"), 16).await;
    write_temp_file(&local_root.join("nested/leaf.bin"), 16).await;
    write_temp_file(&local_root.join("only-local.bin"), 16).await;
    let local_root = local_root.to_string_lossy().into_owned();

    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([
            ("/remote/tree".to_string(), MockEntry::directory(None)),
            (
                "/remote/tree/root.bin".to_string(),
                MockEntry::file(16, None),
            ),
            (
                "/remote/tree/nested".to_string(),
                MockEntry::directory(None),
            ),
            (
                "/remote/tree/nested/leaf.bin".to_string(),
                MockEntry::file(16, None),
            ),
            (
                "/remote/tree/only-remote.bin".to_string(),
                MockEntry::file(16, None),
            ),
        ]),
    );

    let uploading = session
        .transfer_conflicts(TransferDirection::Upload, &local_root, "/remote/tree")
        .await
        .expect("upload conflicts should be listed");
    assert_eq!(uploading, vec!["nested/leaf.bin", "root.bin"]);

    let downloading = session
        .transfer_conflicts(TransferDirection::Download, "/remote/tree", &local_root)
        .await
        .expect("download conflicts should be listed");
    assert_eq!(downloading, vec!["nested/leaf.bin", "root.bin"]);
}

#[tokio::test]
async fn test_transfer_conflicts_empty_when_destination_is_missing() {
    let temp_dir = tempfile::tempdir().expect("temp dir should be created");
    let local_root = temp_dir.path().join("tree");
    tokio::fs::create_dir_all(&local_root)
        .await
        .expect("dir should be created");
    write_temp_file(&local_root.join("root.bin"), 16).await;
    let local_root = local_root.to_string_lossy().into_owned();
    let session = SftpSession::mock("sftp-1");

    let conflicts = session
        .transfer_conflicts(TransferDirection::Upload, &local_root, "/remote/tree")
        .await
        .expect("missing destination should not error");

    assert!(conflicts.is_empty());
}
