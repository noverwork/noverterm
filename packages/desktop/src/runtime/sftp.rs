use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use russh::client::Handle;
use russh_sftp::client::fs::Metadata;
use russh_sftp::client::SftpSession as RusshSftpSession;
use russh_sftp::protocol::FileType as RusshFileType;
use russh_sftp::protocol::OpenFlags;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TransferProgress {
    pub transfer_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
    pub direction: TransferDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TransferComplete {
    pub transfer_id: String,
    pub total_bytes: u64,
    pub direction: TransferDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TransferError {
    pub transfer_id: String,
    pub error: String,
    pub direction: TransferDirection,
}

#[derive(Debug, Clone)]
pub struct TransferCancellation {
    token: Arc<AtomicBool>,
}

impl TransferCancellation {
    pub fn new() -> Self {
        Self {
            token: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.token.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.load(Ordering::SeqCst)
    }
}

impl Default for TransferCancellation {
    fn default() -> Self {
        Self::new()
    }
}

enum SftpSessionInner {
    Active(RusshSftpSession),
    #[cfg(test)]
    Mock {
        close_error: Option<String>,
        entries: std::sync::Mutex<HashMap<String, MockEntry>>,
        upload_result: Result<u64, String>,
        download_result: Result<u64, String>,
        remote_files: std::sync::Mutex<HashMap<String, Vec<u8>>>,
        progress_callbacks: std::sync::Mutex<Vec<TransferProgress>>,
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
                .map_err(|error| classify_sftp_error("close", self.id(), error)),
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

    pub async fn upload(
        &self,
        local_path: &str,
        remote_path: &str,
        transfer_id: String,
        cancel: TransferCancellation,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<u64, SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => {
                upload_sftp(
                    session,
                    local_path,
                    remote_path,
                    transfer_id,
                    cancel,
                    progress_tx,
                )
                .await
            }
            #[cfg(test)]
            SftpSessionInner::Mock {
                entries,
                upload_result,
                progress_callbacks,
                ..
            } => {
                upload_mock(
                    entries,
                    upload_result,
                    progress_callbacks,
                    local_path,
                    remote_path,
                    transfer_id,
                    cancel,
                    progress_tx,
                )
                .await
            }
        }
    }

    pub async fn download(
        &self,
        remote_path: &str,
        local_path: &str,
        transfer_id: String,
        cancel: TransferCancellation,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<u64, SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => {
                download_sftp(
                    session,
                    remote_path,
                    local_path,
                    transfer_id,
                    cancel,
                    progress_tx,
                )
                .await
            }
            #[cfg(test)]
            SftpSessionInner::Mock {
                entries,
                download_result,
                remote_files,
                progress_callbacks,
                ..
            } => {
                download_mock(
                    entries,
                    remote_files,
                    download_result,
                    progress_callbacks,
                    remote_path,
                    local_path,
                    transfer_id,
                    cancel,
                    progress_tx,
                )
                .await
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
                upload_result: Ok(0),
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
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
                upload_result: Ok(0),
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    #[cfg(test)]
    fn mock_with_upload_result(id: impl Into<String>, upload_result: Result<u64, String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::new()),
                upload_result,
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    #[cfg(test)]
    fn mock_with_download_file(
        id: impl Into<String>,
        remote_path: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        let remote_path = remote_path.into();
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::from([(
                    remote_path.clone(),
                    MockEntry::file(data.len() as u64, None),
                )])),
                upload_result: Ok(0),
                download_result: Ok(data.len() as u64),
                remote_files: std::sync::Mutex::new(HashMap::from([(remote_path, data)])),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    #[cfg(test)]
    fn mock_with_download_error(
        id: impl Into<String>,
        remote_path: impl Into<String>,
        data: Vec<u8>,
        error: impl Into<String>,
    ) -> Self {
        let remote_path = remote_path.into();
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::from([(
                    remote_path.clone(),
                    MockEntry::file(data.len() as u64, None),
                )])),
                upload_result: Ok(0),
                download_result: Err(error.into()),
                remote_files: std::sync::Mutex::new(HashMap::from([(remote_path, data)])),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
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
                upload_result: Ok(0),
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
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

    #[cfg(test)]
    fn mock_progress_events(&self) -> Vec<TransferProgress> {
        match &self.inner {
            SftpSessionInner::Active(_) => Vec::new(),
            SftpSessionInner::Mock {
                progress_callbacks, ..
            } => progress_callbacks
                .lock()
                .map(|events| events.clone())
                .unwrap_or_default(),
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
        .map_err(|error| classify_sftp_error("read", path, error))?
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
        .map_err(|error| classify_sftp_error("read", path, error))?;

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
            classify_sftp_error("write", path, message)
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

fn classify_sftp_error(
    operation: &'static str,
    path: &str,
    error: impl ToString,
) -> SftpError {
    let message = error.to_string();
    let lower_message = message.to_lowercase();

    if is_permission_denied(&lower_message) {
        return SftpError::PermissionDenied {
            operation,
            path: path.to_string(),
        };
    }

    if is_connection_lost(&lower_message) {
        return SftpError::ConnectionLost(format!(
            "while trying to {operation} {path}: {message}"
        ));
    }

    SftpError::OperationFailed(format!("cannot {operation} {path}: {message}"))
}

fn is_permission_denied(message: &str) -> bool {
    message.contains("permission denied") || (message.contains("permission") && message.contains("denied"))
}

fn is_connection_lost(message: &str) -> bool {
    message.contains("connection lost")
        || message.contains("connection reset")
        || message.contains("connection closed")
        || message.contains("channel closed")
        || message.contains("transport closed")
        || message.contains("broken pipe")
        || message.contains("unexpected eof")
        || message == "eof"
}

pub async fn remove_sftp(session: &RusshSftpSession, path: &str) -> Result<(), SftpError> {
    let metadata = session
        .metadata(path)
        .await
        .map_err(|error| classify_sftp_error("read", path, error))?;

    match metadata.file_type() {
        RusshFileType::Dir => {
            let mut entries = session
                .read_dir(path)
                .await
                .map_err(|error| classify_sftp_error("read", path, error))?;

            if entries.next().is_some() {
                return Err(SftpError::DirectoryNotEmpty);
            }

            session
                .remove_dir(path)
                .await
                .map_err(|error| classify_sftp_error("write", path, error))
        }
        _ => session.remove_file(path).await.map_err(|error| {
            let message = error.to_string();
            if message.to_lowercase().contains("is a directory") {
                SftpError::IsADirectory
            } else {
                classify_sftp_error("write", path, message)
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
            classify_sftp_error("write", old, message)
        }
    })
}

const UPLOAD_CHUNK_SIZE: usize = 32 * 1024;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const PROGRESS_BYTE_INTERVAL: u64 = 1024 * 1024;

struct ProgressEmitter {
    transfer_id: String,
    total_bytes: u64,
    direction: TransferDirection,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    start: Instant,
    last_emit_at: Instant,
    last_emit_bytes: u64,
}

impl ProgressEmitter {
    fn new(
        transfer_id: String,
        total_bytes: u64,
        direction: TransferDirection,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Self {
        let now = Instant::now();
        Self {
            transfer_id,
            total_bytes,
            direction,
            progress_tx,
            start: now,
            last_emit_at: now,
            last_emit_bytes: 0,
        }
    }

    fn maybe_emit(&mut self, bytes_transferred: u64) -> Option<TransferProgress> {
        if bytes_transferred == self.last_emit_bytes {
            return None;
        }

        let now = Instant::now();
        let enough_time = now.duration_since(self.last_emit_at) >= PROGRESS_INTERVAL;
        let enough_bytes =
            bytes_transferred.saturating_sub(self.last_emit_bytes) >= PROGRESS_BYTE_INTERVAL;
        let complete = bytes_transferred == self.total_bytes;

        if complete || enough_time || enough_bytes {
            return self.emit(bytes_transferred, now);
        }

        None
    }

    fn emit(&mut self, bytes_transferred: u64, now: Instant) -> Option<TransferProgress> {
        let progress = TransferProgress {
            transfer_id: self.transfer_id.clone(),
            bytes_transferred,
            total_bytes: self.total_bytes,
            speed_bps: speed_bps(bytes_transferred, now.duration_since(self.start)),
            direction: self.direction,
        };

        if let Some(progress_tx) = &self.progress_tx {
            let _ = progress_tx.send(progress.clone());
        }

        self.last_emit_at = now;
        self.last_emit_bytes = bytes_transferred;
        Some(progress)
    }
}

fn speed_bps(bytes_transferred: u64, elapsed: Duration) -> u64 {
    let elapsed_secs = elapsed.as_secs_f64();
    if elapsed_secs <= f64::EPSILON {
        return 0;
    }

    (bytes_transferred as f64 / elapsed_secs) as u64
}

pub async fn upload_sftp(
    session: &RusshSftpSession,
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
    let mut remote_file = session
        .open_with_flags(
            remote_path,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
        )
        .await
        .map_err(|error| classify_sftp_error("write", remote_path, error))?;

    let mut progress = ProgressEmitter::new(
        transfer_id,
        total_bytes,
        TransferDirection::Upload,
        progress_tx,
    );
    let mut buffer = vec![0; UPLOAD_CHUNK_SIZE];
    let mut bytes_transferred = 0;

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

            remote_file
                .write_all(&buffer[..read_count])
                .await
                .map_err(|error| classify_sftp_error("write", remote_path, error))?;
            bytes_transferred += read_count as u64;
            progress.maybe_emit(bytes_transferred);
        }

        remote_file
            .flush()
            .await
            .map_err(|error| classify_sftp_error("write", remote_path, error))?;
        remote_file
            .shutdown()
            .await
            .map_err(|error| classify_sftp_error("write", remote_path, error))?;

        progress.maybe_emit(bytes_transferred);
        Ok(bytes_transferred)
    }
    .await;

    if result.is_err() {
        let _ = session.remove_file(remote_path).await;
    }

    result
}

#[cfg(test)]
async fn upload_mock(
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

        if let Some(event) = progress.maybe_emit(bytes_transferred) {
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

pub async fn download_sftp(
    session: &RusshSftpSession,
    remote_path: &str,
    local_path: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let total_bytes = session
        .metadata(remote_path)
        .await
        .map_err(|error| classify_sftp_error("read", remote_path, error))?
        .len();
    let mut remote_file = session
        .open_with_flags(remote_path, OpenFlags::READ)
        .await
        .map_err(|error| classify_sftp_error("read", remote_path, error))?;
    let mut local_file = tokio::fs::File::create(local_path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
    let mut progress = ProgressEmitter::new(
        transfer_id,
        total_bytes,
        TransferDirection::Download,
        progress_tx,
    );
    let mut buffer = vec![0; UPLOAD_CHUNK_SIZE];
    let mut bytes_transferred = 0;

    let result = async {
        loop {
            if cancel.is_cancelled() {
                return Err(SftpError::OperationFailed("download cancelled".to_string()));
            }

            let read_count = remote_file
                .read(&mut buffer)
                .await
                .map_err(|error| classify_sftp_error("read", remote_path, error))?;
            if read_count == 0 {
                break;
            }

            local_file
                .write_all(&buffer[..read_count])
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            bytes_transferred += read_count as u64;
            progress.maybe_emit(bytes_transferred);
        }

        local_file
            .flush()
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        progress.maybe_emit(bytes_transferred);
        Ok(bytes_transferred)
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(local_path).await;
    }

    result
}

#[cfg(test)]
async fn download_mock(
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
        let entry = entries.get(remote_path).copied().ok_or_else(|| {
            SftpError::OperationFailed(format!("SFTP path not found: {remote_path}"))
        })?;

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
        if let Some(event) = progress.maybe_emit(bytes_transferred) {
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
    NotConnected,
    PermissionDenied {
        operation: &'static str,
        path: String,
    },
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
            Self::NotConnected => write!(formatter, "SFTP session is not connected"),
            Self::PermissionDenied { operation, path } => {
                write!(formatter, "Permission denied: cannot {operation} {path}")
            }
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
    use std::path::Path;
    use std::time::Duration;

    use tokio::io::AsyncWriteExt;

    use super::{
        classify_sftp_error, FileEntry, FileType, MockEntry, SftpError, SftpSession,
        SftpSessionManager, TransferCancellation, TransferDirection, UPLOAD_CHUNK_SIZE,
    };

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

    mod upload {
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
        async fn test_upload_error_cleanup() {
            let temp_dir = tempfile::tempdir().expect("temp dir should be created");
            let local_path = temp_dir.path().join("error.bin");
            write_temp_file(&local_path, 1024 * 1024).await;
            let local_path = local_path.to_string_lossy().into_owned();
            let session = SftpSession::mock_with_upload_result(
                "sftp-1",
                Err("remote write failed".to_string()),
            );

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
    }

    mod download {
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
            let session =
                SftpSession::mock_with_download_file("sftp-1", "/remote/small.bin", data.clone());
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
            let session =
                SftpSession::mock_with_download_file("sftp-1", "/remote/large.bin", data.clone());

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
            assert!(callback_events.len() >= 5);
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
            let session =
                SftpSession::mock_with_download_file("sftp-1", "/remote/cancel.bin", data);
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
        assert_eq!(
            error.to_string(),
            "Permission denied: cannot read /root/secret"
        );
    }

    #[test]
    fn test_connection_loss_message_is_specific() {
        let error = classify_sftp_error("write", "/remote/file.txt", "channel closed");

        assert!(matches!(error, SftpError::ConnectionLost(_)));
        assert_eq!(
            error.to_string(),
            "SFTP connection lost: while trying to write /remote/file.txt: channel closed"
        );
    }

    #[test]
    fn test_not_connected_message_is_clear() {
        assert_eq!(
            SftpError::NotConnected.to_string(),
            "SFTP session is not connected"
        );
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
