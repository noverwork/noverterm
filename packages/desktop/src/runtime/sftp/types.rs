use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use serde::{Deserialize, Serialize};

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
    reason: Arc<StdMutex<Option<String>>>,
}

impl TransferCancellation {
    pub fn new() -> Self {
        Self {
            token: Arc::new(AtomicBool::new(false)),
            reason: Arc::new(StdMutex::new(None)),
        }
    }

    pub fn cancel(&self) {
        self.token.store(true, Ordering::SeqCst);
    }

    pub fn cancel_with_reason(&self, reason: impl Into<String>) {
        if let Ok(mut current_reason) = self.reason.lock() {
            *current_reason = Some(reason.into());
        }

        self.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.load(Ordering::SeqCst)
    }

    pub fn reason(&self) -> Option<String> {
        self.reason.lock().ok().and_then(|reason| reason.clone())
    }
}

impl Default for TransferCancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpError {
    AlreadyOpen,
    AlreadyExists,
    ChannelOpenFailed(String),
    DirectoryNotEmpty,
    IsADirectory,
    NotFound(String),
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
            Self::NotFound(path) => write!(formatter, "Path not found: {path}"),
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

pub(super) fn classify_sftp_error(
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

    if is_not_found(&lower_message) {
        return SftpError::NotFound(path.to_string());
    }

    if is_connection_lost(&lower_message) {
        return SftpError::ConnectionLost(format!("while trying to {operation} {path}: {message}"));
    }

    SftpError::OperationFailed(format!("cannot {operation} {path}: {message}"))
}

fn is_permission_denied(message: &str) -> bool {
    message.contains("permission denied")
        || (message.contains("permission") && message.contains("denied"))
}

fn is_not_found(message: &str) -> bool {
    message.contains("not found")
        || message.contains("no such file")
        || message.contains("no such path")
        || message.contains("does not exist")
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
