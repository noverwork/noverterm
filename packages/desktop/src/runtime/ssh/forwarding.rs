use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use russh::client::Handle;
use tauri::{AppHandle, Emitter};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{info, warn};
use uuid::Uuid;

use super::{
    ClientHandler, SshLocalPortForwardInput, SshPortForwardState, SshPortForwardStatus,
    SshPortForwardTask, SshSession, SshSessionManager,
};

impl SshSessionManager {
    pub async fn start_local_port_forward(
        &self,
        app: AppHandle,
        input: SshLocalPortForwardInput,
    ) -> Result<SshPortForwardStatus, String> {
        let input = normalize_port_forward_input(input)?;

        let listener = TcpListener::bind((input.bind_host.as_str(), input.bind_port))
            .await
            .map_err(|e| {
                format!(
                    "Failed to bind local forward on {}:{}: {}",
                    input.bind_host, input.bind_port, e
                )
            })?;
        let bind_port = listener
            .local_addr()
            .map_err(|e| format!("Failed to inspect local forward listener: {e}"))?
            .port();

        let forward_id = Uuid::new_v4().to_string();
        let status = SshPortForwardStatus {
            forward_id: forward_id.clone(),
            session_id: input.session_id.clone(),
            bind_host: input.bind_host,
            bind_port,
            target_host: input.target_host,
            target_port: input.target_port,
            status: SshPortForwardState::Listening,
            error: None,
        };

        let handle = {
            let sessions = self.sessions.lock().await;
            sessions
                .get(&status.session_id)
                .ok_or_else(|| format!("Session not found: {}", status.session_id))?
                .handle
                .clone()
        };

        let task_status = status.clone();
        let task_sessions = self.sessions.clone();
        let task_app = app.clone();
        let task = tokio::spawn(async move {
            run_local_port_forward(listener, handle, task_status, task_app, task_sessions).await;
        });

        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&status.session_id) else {
            task.abort();
            return Err(format!("Session not found: {}", status.session_id));
        };
        session.port_forwards.insert(
            forward_id,
            SshPortForwardTask {
                status: status.clone(),
                task,
            },
        );

        emit_port_forward_event(&app, &status);
        info!(
            session_id = %status.session_id,
            forward_id = %status.forward_id,
            bind_host = %status.bind_host,
            bind_port = status.bind_port,
            target_host = %status.target_host,
            target_port = status.target_port,
            "Started SSH local port forward"
        );
        Ok(status)
    }

    pub async fn stop_port_forward(
        &self,
        app: AppHandle,
        session_id: &str,
        forward_id: &str,
    ) -> Result<SshPortForwardStatus, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        let forward = session
            .port_forwards
            .remove(forward_id)
            .ok_or_else(|| format!("Port forward not found: {forward_id}"))?;

        forward.task.abort();
        let status = stopped_port_forward_status(forward.status, None);
        emit_port_forward_event(&app, &status);
        info!(session_id, forward_id, "Stopped SSH local port forward");
        Ok(status)
    }
}

async fn run_local_port_forward(
    listener: TcpListener,
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    status: SshPortForwardStatus,
    app: AppHandle,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
) {
    let mut connections = JoinSet::new();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((local_stream, originator_addr)) => {
                        let connection_handle = handle.clone();
                        let connection_status = status.clone();
                        connections.spawn(async move {
                            handle_forward_connection(
                                local_stream,
                                originator_addr,
                                connection_handle,
                                connection_status,
                            ).await
                        });
                    }
                    Err(error) => {
                        let error_message = format!("Local forward listener failed: {error}");
                        warn!(
                            session_id = %status.session_id,
                            forward_id = %status.forward_id,
                            "{}",
                            error_message
                        );
                        emit_port_forward_event(&app, &error_port_forward_status(status.clone(), error_message));
                        break;
                    }
                }
            }
            Some(connection_result) = connections.join_next(), if !connections.is_empty() => {
                match connection_result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        warn!(
                            session_id = %status.session_id,
                            forward_id = %status.forward_id,
                            "SSH local port forward connection failed: {}",
                            error
                        );
                        emit_port_forward_event(&app, &error_port_forward_status(status.clone(), error));
                    }
                    Err(error) => {
                        let error_message = format!("Local forward connection task failed: {error}");
                        warn!(
                            session_id = %status.session_id,
                            forward_id = %status.forward_id,
                            "{}",
                            error_message
                        );
                        emit_port_forward_event(&app, &error_port_forward_status(status.clone(), error_message));
                    }
                }
            }
        }
    }

    connections.abort_all();
    let _ = sessions
        .lock()
        .await
        .get_mut(&status.session_id)
        .map(|session| session.port_forwards.remove(&status.forward_id));
}

async fn handle_forward_connection(
    mut local_stream: TcpStream,
    originator_addr: SocketAddr,
    handle: Arc<Mutex<Handle<ClientHandler>>>,
    status: SshPortForwardStatus,
) -> Result<(), String> {
    let channel = handle
        .lock()
        .await
        .channel_open_direct_tcpip(
            status.target_host.clone(),
            u32::from(status.target_port),
            originator_addr.ip().to_string(),
            u32::from(originator_addr.port()),
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to open direct-tcpip channel to {}:{}: {}",
                status.target_host, status.target_port, e
            )
        })?;
    let mut remote_stream = channel.into_stream();
    copy_bidirectional(&mut local_stream, &mut remote_stream)
        .await
        .map_err(|e| format!("Failed to pipe local forward traffic: {e}"))?;
    Ok(())
}

fn normalize_port_forward_input(
    input: SshLocalPortForwardInput,
) -> Result<SshLocalPortForwardInput, String> {
    let session_id = input.session_id.trim().to_string();
    let bind_host = input.bind_host.trim().to_string();
    let target_host = input.target_host.trim().to_string();

    if input.session_id.trim().is_empty() {
        return Err("session_id is required".to_string());
    }
    if bind_host.is_empty() {
        return Err("bind_host is required".to_string());
    }
    if !is_loopback_bind_host(&bind_host) {
        return Err("Local port forwards can only bind to loopback hosts".to_string());
    }
    if target_host.is_empty() {
        return Err("target_host is required".to_string());
    }
    if input.target_port == 0 {
        return Err("target_port must be greater than 0".to_string());
    }

    Ok(SshLocalPortForwardInput {
        session_id,
        bind_host,
        bind_port: input.bind_port,
        target_host,
        target_port: input.target_port,
    })
}

fn is_loopback_bind_host(bind_host: &str) -> bool {
    matches!(bind_host, "127.0.0.1" | "localhost" | "::1")
}

pub(super) fn stopped_port_forward_status(
    mut status: SshPortForwardStatus,
    error: Option<String>,
) -> SshPortForwardStatus {
    status.status = SshPortForwardState::Stopped;
    status.error = error;
    status
}

fn error_port_forward_status(
    mut status: SshPortForwardStatus,
    error: String,
) -> SshPortForwardStatus {
    status.status = SshPortForwardState::Error;
    status.error = Some(error);
    status
}

pub(super) fn emit_port_forward_event(app: &AppHandle, status: &SshPortForwardStatus) {
    if let Err(e) = app.emit("ssh_port_forward", status.clone()) {
        warn!(
            session_id = %status.session_id,
            forward_id = %status.forward_id,
            "Failed to emit ssh_port_forward: {}",
            e
        );
    }
}
