use std::collections::HashMap;
use std::fmt;

use russh::client::Handle;
use russh_sftp::client::fs::Metadata;
use russh_sftp::client::SftpSession as RusshSftpSession;
use russh_sftp::protocol::FileType as RusshFileType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ssh::ClientHandler;

pub struct SftpSession {
    id: String,
    inner: SftpSessionInner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub modified: Option<u64>,
    pub file_type: FileType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum FileType {
    File,
    Dir,
    Symlink,
    Other,
}

enum SftpSessionInner {
    Active(RusshSftpSession),
    #[cfg(test)]
    Mock {
        close_error: Option<String>,
        entries: std::sync::Mutex<HashMap<String, MockEntry>>,
    },
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MockFileType {
    File,
    Directory,
    Symlink,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MockEntry {
    file_type: MockFileType,
    size: u64,
    modified: Option<u64>,
}

#[cfg(test)]
impl MockEntry {
    fn file(size: u64, modified: Option<u64>) -> Self {
        Self {
            file_type: MockFileType::File,
            size,
            modified,
        }
    }

    fn directory(modified: Option<u64>) -> Self {
        Self {
            file_type: MockFileType::Directory,
            size: 0,
            modified,
        }
    }

    fn symlink(size: u64, modified: Option<u64>) -> Self {
        Self {
            file_type: MockFileType::Symlink,
            size,
            modified,
        }
    }
}

impl fmt::Debug for SftpSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SftpSession")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl SftpSession {
    fn new(id: String, inner: RusshSftpSession) -> Self {
        Self {
            id,
            inner: SftpSessionInner::Active(inner),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>, SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => list_sftp_dir(session, path).await,
            #[cfg(test)]
            SftpSessionInner::Mock { entries, .. } => {
                let entries = entries
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
                list_mock_dir(&entries, path)
            }
        }
    }

    pub async fn stat(&self, path: &str) -> Result<FileEntry, SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => stat_sftp(session, path).await,
            #[cfg(test)]
            SftpSessionInner::Mock { entries, .. } => {
                let entries = entries
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
                stat_mock(&entries, path)
            }
        }
    }

    pub async fn mkdir(&self, path: &str) -> Result<(), SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => mkdir_sftp(session, path).await,
            #[cfg(test)]
            SftpSessionInner::Mock { entries, .. } => {
                let mut entries = entries
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
                mkdir_mock(&mut entries, path)
            }
        }
    }

    pub async fn close(&self) -> Result<(), SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => session
                .close()
                .await
                .map_err(|error| SftpError::CloseFailed(error.to_string())),
            #[cfg(test)]
            SftpSessionInner::Mock { close_error, .. } => match close_error {
                Some(error) => Err(SftpError::ConnectionLost(error.clone())),
                None => Ok(()),
            },
        }
    }

    pub async fn remove(&self, path: &str) -> Result<(), SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => remove_sftp(session, path).await,
            #[cfg(test)]
            SftpSessionInner::Mock { entries, .. } => {
                let mut entries = entries
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
                remove_mock(&mut entries, path)
            }
        }
    }

    pub async fn rename(&self, old: &str, new: &str) -> Result<(), SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => rename_sftp(session, old, new).await,
            #[cfg(test)]
            SftpSessionInner::Mock { entries, .. } => {
                let mut entries = entries
                    .lock()
                    .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
                rename_mock(&mut entries, old, new)
            }
        }
    }

    #[cfg(test)]
    fn mock(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::new()),
            },
        }
    }

    #[cfg(test)]
    fn mock_with_entries(id: impl Into<String>, entries: HashMap<String, MockEntry>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(entries),
            },
        }
    }

    #[cfg(test)]
    fn mock_with_close_error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: Some(error.into()),
                entries: std::sync::Mutex::new(HashMap::new()),
            },
        }
    }

    #[cfg(test)]
    fn mock_contains_path(&self, path: &str) -> bool {
        match &self.inner {
            SftpSessionInner::Active(_) => false,
            SftpSessionInner::Mock { entries, .. } => entries
                .lock()
                .map(|entries| entries.contains_key(path))
                .unwrap_or(false),
        }
    }
}

#[derive(Default, Debug)]
pub struct SftpSessionManager {
    sessions: HashMap<String, SftpSession>,
}

impl SftpSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    pub async fn close(&mut self, session_id: &str) -> Result<(), SftpError> {
        close_sftp_session(&mut self.sessions, session_id).await
    }

    #[cfg(test)]
    fn insert(&mut self, session: SftpSession) -> Result<String, SftpError> {
        if !self.sessions.is_empty() {
            return Err(SftpError::AlreadyOpen);
        }

        let session_id = session.id().to_string();
        self.sessions.insert(session_id.clone(), session);
        Ok(session_id)
    }
}

pub(crate) async fn open_sftp_session(
    handle: &mut Handle<ClientHandler>,
) -> Result<SftpSession, SftpError> {
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|error| SftpError::ChannelOpenFailed(error.to_string()))?;

    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|error| SftpError::SubsystemNotSupported(error.to_string()))?;

    let session = RusshSftpSession::new(channel.into_stream())
        .await
        .map_err(|error| SftpError::SessionInitFailed(error.to_string()))?;

    Ok(SftpSession::new(Uuid::new_v4().to_string(), session))
}

pub async fn close_sftp_session(
    sessions: &mut HashMap<String, SftpSession>,
    session_id: &str,
) -> Result<(), SftpError> {
    let session = sessions
        .remove(session_id)
        .ok_or_else(|| SftpError::SessionNotFound(session_id.to_string()))?;

    if let Err(error) = session.close().await {
        sessions.insert(session_id.to_string(), session);
        return Err(error);
    }

    Ok(())
}

pub async fn list_sftp_dir(
    session: &RusshSftpSession,
    path: &str,
) -> Result<Vec<FileEntry>, SftpError> {
    let mut entries = session
        .read_dir(path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?
        .filter(|entry| {
            let name = entry.file_name();
            name != "." && name != ".."
        })
        .map(|entry| file_entry_from_metadata(entry.file_name(), &entry.metadata()))
        .collect::<Vec<_>>();

    sort_file_entries(&mut entries);
    Ok(entries)
}

pub async fn stat_sftp(session: &RusshSftpSession, path: &str) -> Result<FileEntry, SftpError> {
    let metadata = session
        .metadata(path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;

    Ok(file_entry_from_metadata(
        file_name_from_path(path),
        &metadata,
    ))
}

pub async fn mkdir_sftp(session: &RusshSftpSession, path: &str) -> Result<(), SftpError> {
    session.create_dir(path).await.map_err(|error| {
        let message = error.to_string();
        if message.to_lowercase().contains("exists") {
            SftpError::AlreadyExists
        } else {
            SftpError::OperationFailed(message)
        }
    })
}

fn file_entry_from_metadata(name: String, metadata: &Metadata) -> FileEntry {
    FileEntry {
        name,
        size: metadata.len(),
        modified: metadata.mtime.map(u64::from),
        file_type: map_file_type(metadata.file_type()),
    }
}

fn map_file_type(file_type: RusshFileType) -> FileType {
    match file_type {
        RusshFileType::File => FileType::File,
        RusshFileType::Dir => FileType::Dir,
        RusshFileType::Symlink => FileType::Symlink,
        RusshFileType::Other => FileType::Other,
    }
}

fn file_name_from_path(path: &str) -> String {
    path.rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn sort_file_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|left, right| {
        file_type_sort_rank(left.file_type)
            .cmp(&file_type_sort_rank(right.file_type))
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn file_type_sort_rank(file_type: FileType) -> u8 {
    match file_type {
        FileType::Dir => 0,
        FileType::File | FileType::Symlink | FileType::Other => 1,
    }
}

pub async fn remove_sftp(session: &RusshSftpSession, path: &str) -> Result<(), SftpError> {
    let metadata = session
        .metadata(path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;

    match metadata.file_type() {
        RusshFileType::Dir => {
            let mut entries = session
                .read_dir(path)
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;

            if entries.next().is_some() {
                return Err(SftpError::DirectoryNotEmpty);
            }

            session
                .remove_dir(path)
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))
        }
        _ => session.remove_file(path).await.map_err(|error| {
            let message = error.to_string();
            if message.to_lowercase().contains("is a directory") {
                SftpError::IsADirectory
            } else {
                SftpError::OperationFailed(message)
            }
        }),
    }
}

pub async fn rename_sftp(
    session: &RusshSftpSession,
    old: &str,
    new: &str,
) -> Result<(), SftpError> {
    if session.metadata(new).await.is_ok() {
        return Err(SftpError::AlreadyExists);
    }

    session.rename(old, new).await.map_err(|error| {
        let message = error.to_string();
        if message.to_lowercase().contains("exists") {
            SftpError::AlreadyExists
        } else {
            SftpError::OperationFailed(message)
        }
    })
}

#[cfg(test)]
fn list_mock_dir(
    entries: &HashMap<String, MockEntry>,
    path: &str,
) -> Result<Vec<FileEntry>, SftpError> {
    let directory = entries
        .get(path)
        .ok_or_else(|| SftpError::OperationFailed(format!("SFTP path not found: {path}")))?;

    if directory.file_type != MockFileType::Directory {
        return Err(SftpError::IsADirectory);
    }

    let mut children = entries
        .iter()
        .filter_map(|(entry_path, entry)| {
            mock_child_name(path, entry_path).map(|name| (name, entry))
        })
        .filter(|(name, _)| name != "." && name != "..")
        .map(|(name, entry)| file_entry_from_mock(name, *entry))
        .collect::<Vec<_>>();

    sort_file_entries(&mut children);
    Ok(children)
}

#[cfg(test)]
fn stat_mock(entries: &HashMap<String, MockEntry>, path: &str) -> Result<FileEntry, SftpError> {
    entries
        .get(path)
        .copied()
        .map(|entry| file_entry_from_mock(file_name_from_path(path), entry))
        .ok_or_else(|| SftpError::OperationFailed(format!("SFTP path not found: {path}")))
}

#[cfg(test)]
fn mkdir_mock(entries: &mut HashMap<String, MockEntry>, path: &str) -> Result<(), SftpError> {
    if entries.contains_key(path) {
        return Err(SftpError::AlreadyExists);
    }

    entries.insert(path.to_string(), MockEntry::directory(None));
    Ok(())
}

#[cfg(test)]
fn mock_child_name(parent: &str, path: &str) -> Option<String> {
    if parent == path {
        return None;
    }

    let prefix = if parent == "/" {
        "/".to_string()
    } else {
        format!("{}/", parent.trim_end_matches('/'))
    };

    let child = path.strip_prefix(&prefix)?;
    if child.is_empty() || child.contains('/') {
        return None;
    }

    Some(child.to_string())
}

#[cfg(test)]
fn file_entry_from_mock(name: String, entry: MockEntry) -> FileEntry {
    FileEntry {
        name,
        size: entry.size,
        modified: entry.modified,
        file_type: mock_file_type(entry.file_type),
    }
}

#[cfg(test)]
fn mock_file_type(file_type: MockFileType) -> FileType {
    match file_type {
        MockFileType::File => FileType::File,
        MockFileType::Directory => FileType::Dir,
        MockFileType::Symlink => FileType::Symlink,
    }
}

#[cfg(test)]
fn remove_mock(entries: &mut HashMap<String, MockEntry>, path: &str) -> Result<(), SftpError> {
    let entry = entries
        .get(path)
        .copied()
        .ok_or_else(|| SftpError::OperationFailed(format!("SFTP path not found: {path}")))?;

    match entry.file_type {
        MockFileType::File | MockFileType::Symlink => {
            entries.remove(path);
            Ok(())
        }
        MockFileType::Directory => {
            if mock_dir_has_entries(entries, path) {
                return Err(SftpError::DirectoryNotEmpty);
            }

            entries.remove(path);
            Ok(())
        }
    }
}

#[cfg(test)]
fn rename_mock(
    entries: &mut HashMap<String, MockEntry>,
    old: &str,
    new: &str,
) -> Result<(), SftpError> {
    if entries.contains_key(new) {
        return Err(SftpError::AlreadyExists);
    }

    if !entries.contains_key(old) {
        return Err(SftpError::OperationFailed(format!(
            "SFTP path not found: {old}"
        )));
    }

    let old_prefix = format!("{}/", old.trim_end_matches('/'));
    let moves = entries
        .keys()
        .filter(|path| *path == old || path.starts_with(&old_prefix))
        .cloned()
        .collect::<Vec<_>>();

    for source in moves {
        if let Some(entry) = entries.remove(&source) {
            let target = if source == old {
                new.to_string()
            } else {
                format!("{}{}", new.trim_end_matches('/'), &source[old.len()..])
            };
            entries.insert(target, entry);
        }
    }

    Ok(())
}

#[cfg(test)]
fn mock_dir_has_entries(entries: &HashMap<String, MockEntry>, path: &str) -> bool {
    let prefix = format!("{}/", path.trim_end_matches('/'));
    entries
        .keys()
        .any(|entry_path| entry_path.starts_with(&prefix))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpError {
    AlreadyOpen,
    AlreadyExists,
    ChannelOpenFailed(String),
    DirectoryNotEmpty,
    IsADirectory,
    SubsystemNotSupported(String),
    SessionInitFailed(String),
    SessionNotFound(String),
    CloseFailed(String),
    ConnectionLost(String),
    OperationFailed(String),
}

impl fmt::Display for SftpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyOpen => write!(formatter, "SFTP session already open"),
            Self::AlreadyExists => write!(formatter, "SFTP destination already exists"),
            Self::ChannelOpenFailed(error) => {
                write!(formatter, "Failed to open SFTP channel: {error}")
            }
            Self::DirectoryNotEmpty => write!(formatter, "SFTP directory is not empty"),
            Self::IsADirectory => write!(formatter, "SFTP path is a directory"),
            Self::SubsystemNotSupported(error) => {
                write!(formatter, "SFTP subsystem is not supported: {error}")
            }
            Self::SessionInitFailed(error) => {
                write!(formatter, "Failed to initialize SFTP session: {error}")
            }
            Self::SessionNotFound(session_id) => {
                write!(formatter, "SFTP session not found: {session_id}")
            }
            Self::CloseFailed(error) => write!(formatter, "Failed to close SFTP session: {error}"),
            Self::ConnectionLost(error) => write!(formatter, "SFTP connection lost: {error}"),
            Self::OperationFailed(error) => write!(formatter, "SFTP operation failed: {error}"),
        }
    }
}

impl std::error::Error for SftpError {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{FileEntry, FileType, MockEntry, SftpError, SftpSession, SftpSessionManager};

    fn mock_file() -> MockEntry {
        MockEntry::file(0, None)
    }

    fn mock_dir() -> MockEntry {
        MockEntry::directory(None)
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

        assert!(result.is_err());
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
}
