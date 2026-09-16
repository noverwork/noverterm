use std::collections::HashMap;
use std::net::Shutdown;
use std::sync::Arc;

use russh::{client, Disconnect};
use russh_sftp::client::SftpSession as RusshSftpSession;
use tauri::AppHandle;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::info;
use uuid::Uuid;

use crate::runtime::sftp::{
    open_sftp_session, sftp_client_config, FileEntry, SftpSession, TransferCancellation,
    TransferDirection, TransferProgress,
};
use crate::trust::SshTrustStore;

use super::authentication::{authenticate_session, client_config, map_connect_error};
use super::{
    keepalive_loop, AuthMethod, ClientHandler, SshConnectResponse, SshSession, SshSessionManager,
    SSH_KEEPALIVE_TIMEOUT,
};

#[cfg(test)]
mod tests;

// russh's handle only queues disconnects and does not abort its transport on drop.
// Keep a direct-only socket handle so stale or blocked transports can be shut down.
pub(super) struct DirectSftpTransport(std::net::TcpStream);

impl DirectSftpTransport {
    async fn connect(host: &str, port: u16, nodelay: bool) -> std::io::Result<(TcpStream, Self)> {
        let stream = TcpStream::connect((host, port)).await?;
        stream.set_nodelay(nodelay)?;
        let stream = stream.into_std()?;
        let transport = Self(stream.try_clone()?);
        Ok((TcpStream::from_std(stream)?, transport))
    }

    fn shutdown(&self) -> std::io::Result<()> {
        match self.0.shutdown(Shutdown::Both) {
            Err(error) if error.kind() == std::io::ErrorKind::NotConnected => Ok(()),
            result => result,
        }
    }
}

impl Drop for DirectSftpTransport {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl SshSessionManager {
    pub async fn open_sftp(&self, session_id: &str) -> Result<String, String> {
        let handle = {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| format!("Session not found: {session_id}"))?;
            if !session.sftp_sessions.is_empty() {
                return Err("SFTP session already open".to_string());
            }
            session.handle.clone()
        };
        let sftp_session = {
            let mut handle = handle.lock().await;
            open_sftp_session(&mut handle)
                .await
                .map_err(|error| error.to_string())?
        };
        let sftp_id = sftp_session.id().to_string();
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        if !session.sftp_sessions.is_empty() {
            return Err("SFTP session already open".to_string());
        }
        session
            .sftp_sessions
            .insert(sftp_id.clone(), Arc::new(sftp_session));
        Ok(sftp_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn connect_direct_sftp(
        &self,
        app: AppHandle,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        private_key: Option<&str>,
        passphrase: Option<&str>,
        trust_store: SshTrustStore,
    ) -> Result<SshConnectResponse, String> {
        let sftp_session_id = Uuid::new_v4().to_string();
        info!(
            sftp_session_id,
            host, port, username, "Starting direct SFTP connection"
        );

        let config = client_config(None);
        let trust_check = Arc::new(Mutex::new(None));
        let handler = ClientHandler::new(host.to_string(), port, trust_store, trust_check.clone());

        let (stream, transport) = DirectSftpTransport::connect(host, port, config.nodelay)
            .await
            .map_err(|error| format!("Failed to connect SFTP transport: {error}"))?;
        let mut session = match client::connect_stream(config, stream, handler).await {
            Ok(session) => session,
            Err(error) => return map_connect_error(error, trust_check).await,
        };

        let auth_method = if let Some(key_content) = private_key {
            AuthMethod::PublicKey {
                private_key: key_content.to_string(),
                passphrase: passphrase.map(|s| s.to_string()),
            }
        } else if let Some(pwd) = password {
            AuthMethod::Password(pwd.to_string())
        } else {
            return Err("No authentication method provided".to_string());
        };

        authenticate_session(&mut session, username, auth_method, &sftp_session_id).await?;

        info!(sftp_session_id, "SFTP authentication successful");

        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| format!("Failed to open channel: {}", e))?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("Failed to request SFTP subsystem: {}", e))?;

        info!(sftp_session_id, "SFTP subsystem requested");

        let sftp = RusshSftpSession::new_with_config(channel.into_stream(), sftp_client_config())
            .await
            .map_err(|e| format!("Failed to create SFTP session: {}", e))?;

        let sftp_session = SftpSession::new(sftp_session_id.clone(), sftp);

        let session_handle = Arc::new(Mutex::new(session));
        let keepalive_handle = session_handle.clone();
        let keepalive_sessions = self.sessions.clone();
        let keepalive_app = app.clone();
        let ka_session_id = sftp_session_id.clone();

        let keepalive_task = tokio::spawn(async move {
            keepalive_loop(
                keepalive_handle,
                ka_session_id,
                keepalive_app,
                keepalive_sessions,
                None,
            )
            .await;
        });

        self.sessions.lock().await.insert(
            sftp_session_id.clone(),
            SshSession {
                handle: session_handle,
                write_tx: None,
                sftp_sessions: HashMap::from([(sftp_session_id.clone(), Arc::new(sftp_session))]),
                port_forwards: HashMap::new(),
                keepalive_task: Some(keepalive_task),
                writer_task: None,
                output: None,
                direct_sftp_transport: Some(transport),
            },
        );

        info!(sftp_session_id, "Direct SFTP session established");
        Ok(SshConnectResponse::Connected {
            session_id: sftp_session_id,
        })
    }

    pub async fn close_sftp(&self, sftp_id: &str) -> Result<(), String> {
        let detached = {
            let mut sessions = self.sessions.lock().await;
            remove_sftp_session(&mut sessions, sftp_id)
        };
        let Some((sftp_session, direct_session)) = detached else {
            return Ok(());
        };

        let sftp_result = sftp_session
            .close()
            .await
            .map_err(|error| error.to_string());
        if let Some(mut session) = direct_session {
            session.stop_runtime_tasks(None, true, true);
            let disconnect_result = timeout(SSH_KEEPALIVE_TIMEOUT, async {
                let mut handle = session.handle.lock().await;
                handle
                    .disconnect(Disconnect::ByApplication, "SFTP disconnected", "")
                    .await?;
                // russh completes a locally initiated disconnect with Error::Disconnect.
                match (&mut *handle).await {
                    Ok(()) | Err(russh::Error::Disconnect) => Ok(()),
                    Err(error) => Err(error),
                }
            })
            .await
            .map_err(|_| "SFTP transport disconnect timed out".to_string())
            .and_then(|result| {
                result.map_err(|error| format!("SFTP transport disconnect failed: {error}"))
            });
            let shutdown_result = session
                .direct_sftp_transport
                .as_ref()
                .map_or(Ok(()), DirectSftpTransport::shutdown)
                .map_err(|error| format!("SFTP transport shutdown failed: {error}"));
            return sftp_result.and(disconnect_result).and(shutdown_result);
        }

        sftp_result
    }

    pub async fn sftp_list_dir(&self, sftp_id: &str, path: &str) -> Result<Vec<FileEntry>, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .list_dir(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_home_dir(&self, sftp_id: &str) -> Result<String, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .home_dir()
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_stat(&self, sftp_id: &str, path: &str) -> Result<FileEntry, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .stat(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_mkdir(&self, sftp_id: &str, path: &str) -> Result<(), String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .mkdir(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_remove(&self, sftp_id: &str, path: &str) -> Result<(), String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .remove(path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_rename(&self, sftp_id: &str, old: &str, new: &str) -> Result<(), String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .rename(old, new)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_transfer_conflicts(
        &self,
        sftp_id: &str,
        direction: TransferDirection,
        source_path: &str,
        target_path: &str,
    ) -> Result<Vec<String>, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .transfer_conflicts(direction, source_path, target_path)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_upload(
        &self,
        sftp_id: &str,
        local_path: &str,
        remote_path: &str,
        transfer_id: String,
        cancel: TransferCancellation,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<u64, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .upload(local_path, remote_path, transfer_id, cancel, progress_tx)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn sftp_download(
        &self,
        sftp_id: &str,
        remote_path: &str,
        local_path: &str,
        transfer_id: String,
        cancel: TransferCancellation,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Result<u64, String> {
        let sftp_session = self.sftp_session(sftp_id).await?;

        sftp_session
            .download(remote_path, local_path, transfer_id, cancel, progress_tx)
            .await
            .map_err(|error| error.to_string())
    }

    async fn sftp_session(&self, sftp_id: &str) -> Result<Arc<SftpSession>, String> {
        let sessions = self.sessions.lock().await;
        find_sftp_session(&sessions, sftp_id)
    }

    pub async fn contains_sftp_session(&self, sftp_id: &str) -> bool {
        self.sessions
            .lock()
            .await
            .values()
            .any(|session| session.sftp_sessions.contains_key(sftp_id))
    }
}

fn find_sftp_session(
    sessions: &HashMap<String, SshSession>,
    sftp_id: &str,
) -> Result<Arc<SftpSession>, String> {
    sessions
        .values()
        .find_map(|session| session.sftp_sessions.get(sftp_id).cloned())
        .ok_or_else(|| format!("SFTP session not found: {sftp_id}"))
}

fn remove_sftp_session(
    sessions: &mut HashMap<String, SshSession>,
    sftp_id: &str,
) -> Option<(Arc<SftpSession>, Option<SshSession>)> {
    let (ssh_id, sftp_session, direct) = sessions.iter_mut().find_map(|(ssh_id, session)| {
        session.sftp_sessions.remove(sftp_id).map(|sftp_session| {
            (
                ssh_id.clone(),
                sftp_session,
                session.direct_sftp_transport.is_some() && session.sftp_sessions.is_empty(),
            )
        })
    })?;
    let parent = if direct {
        sessions.remove(&ssh_id)
    } else {
        None
    };
    Some((sftp_session, parent))
}
