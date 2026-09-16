use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use tokio::io::AsyncWriteExt;

use super::mock::MockEntry;
use super::transfer::UPLOAD_CHUNK_SIZE;
use super::types::classify_sftp_error;
use super::{
    FileEntry, FileType, SftpError, SftpSession, SftpSessionManager, TransferCancellation,
    TransferDirection,
};

mod download;
mod tree;
mod upload;

fn mock_file() -> MockEntry {
    MockEntry::file(0, None)
}

fn mock_dir() -> MockEntry {
    MockEntry::directory(None)
}

async fn write_temp_file(path: &Path, size: usize) {
    let mut file = tokio::fs::File::create(path)
        .await
        .expect("temp file should be created");
    let chunk = vec![0xa5; UPLOAD_CHUNK_SIZE];
    let mut remaining = size;

    while remaining > 0 {
        let write_size = remaining.min(chunk.len());
        file.write_all(&chunk[..write_size])
            .await
            .expect("temp file chunk should be written");
        remaining -= write_size;
    }
}

#[tokio::test]
async fn test_open_sftp_session() {
    let mut manager = SftpSessionManager::new();

    let session_id = manager
        .insert(SftpSession::mock("sftp-1"))
        .expect("mock SFTP session should open");

    assert_eq!(session_id, "sftp-1");
    assert_eq!(manager.len(), 1);
    assert!(manager.contains("sftp-1"));
}

#[tokio::test]
async fn test_close_sftp_session() {
    let mut manager = SftpSessionManager::new();
    manager
        .insert(SftpSession::mock("sftp-1"))
        .expect("mock SFTP session should open");

    manager
        .close("sftp-1")
        .await
        .expect("mock SFTP session should close");

    assert!(manager.is_empty());
    assert!(!manager.contains("sftp-1"));
}

#[tokio::test]
async fn test_double_open_sftp_prevented() {
    let mut manager = SftpSessionManager::new();
    manager
        .insert(SftpSession::mock("sftp-1"))
        .expect("first mock SFTP session should open");

    let result = manager.insert(SftpSession::mock("sftp-2"));

    assert_eq!(result, Err(SftpError::AlreadyOpen));
    assert_eq!(manager.len(), 1);
    assert!(manager.contains("sftp-1"));
}

#[tokio::test]
async fn test_close_nonexistent_sftp() {
    let mut manager = SftpSessionManager::new();

    let result = manager.close("missing").await;

    assert_eq!(
        result,
        Err(SftpError::SessionNotFound("missing".to_string()))
    );
}

#[tokio::test]
async fn test_sftp_error_handling() {
    let mut manager = SftpSessionManager::new();
    manager
        .insert(SftpSession::mock_with_close_error(
            "sftp-1",
            "transport closed",
        ))
        .expect("mock SFTP session should open");

    let result = manager.close("sftp-1").await;

    assert_eq!(
        result,
        Err(SftpError::ConnectionLost("transport closed".to_string()))
    );
    assert!(manager.contains("sftp-1"));
}

#[test]
fn test_permission_denied_message_includes_operation_and_path() {
    let error = classify_sftp_error("read", "/root/secret", "permission denied");

    assert_eq!(
        error,
        SftpError::PermissionDenied {
            operation: "read",
            path: "/root/secret".to_string(),
        }
    );
}

#[test]
fn test_connection_loss_message_is_specific() {
    let error = classify_sftp_error("write", "/remote/file.txt", "channel closed");

    assert!(matches!(error, SftpError::ConnectionLost(_)));
}

#[test]
fn test_not_found_classification_message_is_clear() {
    let error = classify_sftp_error("read", "/missing/file.txt", "not found");

    assert_eq!(error, SftpError::NotFound("/missing/file.txt".to_string()));
}

#[tokio::test]
async fn test_list_dir_mixed() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([
            ("/remote".to_string(), MockEntry::directory(Some(10))),
            (
                "/remote/zeta.txt".to_string(),
                MockEntry::file(120, Some(30)),
            ),
            ("/remote/docs".to_string(), MockEntry::directory(Some(20))),
            ("/remote/link".to_string(), MockEntry::symlink(9, Some(40))),
            ("/remote/.".to_string(), MockEntry::directory(None)),
            ("/remote/..".to_string(), MockEntry::directory(None)),
        ]),
    );

    let entries = session
        .list_dir("/remote")
        .await
        .expect("mock directory should list");

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].name, "docs");
    assert_eq!(entries[0].file_type, FileType::Dir);
    assert_eq!(entries[1].name, "link");
    assert_eq!(entries[1].file_type, FileType::Symlink);
    assert_eq!(entries[2].name, "zeta.txt");
    assert_eq!(entries[2].file_type, FileType::File);
}

#[tokio::test]
async fn test_list_empty_dir() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/empty".to_string(), MockEntry::directory(Some(10)))]),
    );

    let entries = session
        .list_dir("/empty")
        .await
        .expect("mock empty directory should list");

    assert!(entries.is_empty());
}

#[tokio::test]
async fn test_stat_file() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([(
            "/remote/readme.md".to_string(),
            MockEntry::file(42, Some(1_700_000_000)),
        )]),
    );

    let entry = session
        .stat("/remote/readme.md")
        .await
        .expect("mock file should stat");

    assert_eq!(
        entry,
        FileEntry {
            name: "readme.md".to_string(),
            size: 42,
            modified: Some(1_700_000_000),
            file_type: FileType::File,
        }
    );
}

#[tokio::test]
async fn test_stat_dir() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([(
            "/remote/docs".to_string(),
            MockEntry::directory(Some(1_700_000_100)),
        )]),
    );

    let entry = session
        .stat("/remote/docs")
        .await
        .expect("mock directory should stat");

    assert_eq!(entry.name, "docs");
    assert_eq!(entry.size, 0);
    assert_eq!(entry.modified, Some(1_700_000_100));
    assert_eq!(entry.file_type, FileType::Dir);
}

#[tokio::test]
async fn test_stat_not_found() {
    let session = SftpSession::mock_with_entries("sftp-1", HashMap::new());

    let result = session.stat("/missing").await;

    assert_eq!(result, Err(SftpError::NotFound("/missing".to_string())));
}

#[tokio::test]
async fn test_mkdir_new() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/remote".to_string(), MockEntry::directory(None))]),
    );

    session
        .mkdir("/remote/new")
        .await
        .expect("mock directory should be created");

    let entry = session
        .stat("/remote/new")
        .await
        .expect("created mock directory should stat");
    assert_eq!(entry.name, "new");
    assert_eq!(entry.file_type, FileType::Dir);
}

#[tokio::test]
async fn test_mkdir_existing() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/remote/existing".to_string(), MockEntry::directory(None))]),
    );

    let result = session.mkdir("/remote/existing").await;

    assert_eq!(result, Err(SftpError::AlreadyExists));
}

#[tokio::test]
async fn test_list_sort_order() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([
            ("/remote".to_string(), MockEntry::directory(None)),
            ("/remote/zfile.txt".to_string(), MockEntry::file(1, None)),
            ("/remote/bdir".to_string(), MockEntry::directory(None)),
            ("/remote/afile.txt".to_string(), MockEntry::file(1, None)),
            ("/remote/adir".to_string(), MockEntry::directory(None)),
        ]),
    );

    let entries = session
        .list_dir("/remote")
        .await
        .expect("mock directory should list");
    let names = entries
        .into_iter()
        .map(|entry| entry.name)
        .collect::<Vec<_>>();

    assert_eq!(names, ["adir", "bdir", "afile.txt", "zfile.txt"]);
}

#[tokio::test]
async fn test_remove_file() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/remote/file.txt".to_string(), mock_file())]),
    );

    session
        .remove("/remote/file.txt")
        .await
        .expect("mock SFTP file should be removed");

    assert!(!session.mock_contains_path("/remote/file.txt"));
}

#[tokio::test]
async fn test_remove_empty_dir() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/remote/empty".to_string(), mock_dir())]),
    );

    session
        .remove("/remote/empty")
        .await
        .expect("mock SFTP empty directory should be removed");

    assert!(!session.mock_contains_path("/remote/empty"));
}

#[tokio::test]
async fn test_remove_nonempty_dir() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([
            ("/remote/dir".to_string(), mock_dir()),
            ("/remote/dir/file.txt".to_string(), mock_file()),
        ]),
    );

    let result = session.remove("/remote/dir").await;

    assert_eq!(result, Err(SftpError::DirectoryNotEmpty));
    assert!(session.mock_contains_path("/remote/dir"));
    assert!(session.mock_contains_path("/remote/dir/file.txt"));
}

#[tokio::test]
async fn test_rename_file() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([("/remote/old.txt".to_string(), mock_file())]),
    );

    session
        .rename("/remote/old.txt", "/remote/new.txt")
        .await
        .expect("mock SFTP file should be renamed");

    assert!(!session.mock_contains_path("/remote/old.txt"));
    assert!(session.mock_contains_path("/remote/new.txt"));
}

#[tokio::test]
async fn test_rename_to_existing() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([
            ("/remote/old.txt".to_string(), mock_file()),
            ("/remote/new.txt".to_string(), mock_file()),
        ]),
    );

    let result = session.rename("/remote/old.txt", "/remote/new.txt").await;

    assert_eq!(result, Err(SftpError::AlreadyExists));
    assert!(session.mock_contains_path("/remote/old.txt"));
    assert!(session.mock_contains_path("/remote/new.txt"));
}

#[tokio::test]
async fn test_rename_dir() {
    let session = SftpSession::mock_with_entries(
        "sftp-1",
        HashMap::from([
            ("/remote/old-dir".to_string(), mock_dir()),
            ("/remote/old-dir/file.txt".to_string(), mock_file()),
        ]),
    );

    session
        .rename("/remote/old-dir", "/remote/new-dir")
        .await
        .expect("mock SFTP directory should be renamed");

    assert!(!session.mock_contains_path("/remote/old-dir"));
    assert!(!session.mock_contains_path("/remote/old-dir/file.txt"));
    assert!(session.mock_contains_path("/remote/new-dir"));
    assert!(session.mock_contains_path("/remote/new-dir/file.txt"));
}
