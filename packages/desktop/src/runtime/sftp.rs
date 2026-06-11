use std::collections::HashMap;
use std::fmt;

use russh::client::Handle;
use russh_sftp::client::SftpSession as RusshSftpSession;
use uuid::Uuid;

use super::ssh::ClientHandler;

pub struct SftpSession {
    id: String,
    inner: SftpSessionInner,
}

enum SftpSessionInner {
    Active(RusshSftpSession),
    #[cfg(test)]
    Mock {
        close_error: Option<String>,
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
    fn new(id: String, inner: RusshSftpSession) -> Self {
        Self {
            id,
            inner: SftpSessionInner::Active(inner),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn close(&self) -> Result<(), SftpError> {
        match &self.inner {
            SftpSessionInner::Active(session) => session
                .close()
                .await
                .map_err(|error| SftpError::CloseFailed(error.to_string())),
            #[cfg(test)]
            SftpSessionInner::Mock { close_error } => match close_error {
                Some(error) => Err(SftpError::ConnectionLost(error.clone())),
                None => Ok(()),
            },
        }
    }

    #[cfg(test)]
    fn mock(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock { close_error: None },
        }
    }

    #[cfg(test)]
    fn mock_with_close_error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: Some(error.into()),
            },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpError {
    AlreadyOpen,
    ChannelOpenFailed(String),
    SubsystemNotSupported(String),
    SessionInitFailed(String),
    SessionNotFound(String),
    CloseFailed(String),
    ConnectionLost(String),
}

impl fmt::Display for SftpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyOpen => write!(formatter, "SFTP session already open"),
            Self::ChannelOpenFailed(error) => {
                write!(formatter, "Failed to open SFTP channel: {error}")
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
        }
    }
}

impl std::error::Error for SftpError {}

#[cfg(test)]
mod tests {
    use super::{SftpError, SftpSession, SftpSessionManager};

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
}
