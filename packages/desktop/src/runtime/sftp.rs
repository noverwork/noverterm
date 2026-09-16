mod filesystem;
mod io;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod transfer;
mod tree;
mod types;

use std::collections::HashMap;
use std::fmt;

use russh::client::Handle;
use russh_sftp::client::SftpSession as RusshSftpSession;
use uuid::Uuid;

use super::ssh::ClientHandler;
#[cfg(test)]
use mock::{
    download_mock, list_mock_dir, mkdir_mock, remove_mock, rename_mock, stat_mock, upload_mock,
    MockEntry,
};
use tree::{download_dir, is_local_dir, upload_dir};
use types::classify_sftp_error;

pub use filesystem::{list_sftp_dir, mkdir_sftp, remove_sftp, rename_sftp, stat_sftp};
pub(crate) use transfer::sftp_client_config;
pub use transfer::{download_sftp, upload_sftp};
pub use types::{
    FileEntry, FileType, SftpError, TransferCancellation, TransferComplete, TransferDirection,
    TransferError, TransferProgress,
};

pub struct SftpSession {
    id: String,
    inner: SftpSessionInner,
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

impl fmt::Debug for SftpSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SftpSession")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl SftpSession {
    pub fn new(id: String, inner: RusshSftpSession) -> Self {
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

    pub async fn home_dir(&self) -> Result<String, SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => session
                .canonicalize(".")
                .await
                .map_err(|error| classify_sftp_error("realpath", ".", error)),
            #[cfg(test)]
            SftpSessionInner::Mock { .. } => Ok("/mock/home".to_string()),
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
        if is_local_dir(local_path).await {
            return upload_dir(
                self,
                local_path,
                remote_path,
                transfer_id,
                cancel,
                progress_tx,
            )
            .await;
        }

        self.upload_file(local_path, remote_path, transfer_id, cancel, progress_tx)
            .await
    }

    async fn upload_file(
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
        if self.stat(remote_path).await?.file_type == FileType::Dir {
            return download_dir(
                self,
                remote_path,
                local_path,
                transfer_id,
                cancel,
                progress_tx,
            )
            .await;
        }

        self.download_file(remote_path, local_path, transfer_id, cancel, progress_tx)
            .await
    }

    async fn download_file(
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

    let session = RusshSftpSession::new_with_config(channel.into_stream(), sftp_client_config())
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
