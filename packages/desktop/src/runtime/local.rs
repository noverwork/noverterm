use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

const LOCAL_WRITE_QUEUE_CAPACITY: usize = 1024;
const LOCAL_WRITE_MAX_BYTES: usize = 1024 * 1024;

pub struct LocalSession {
    #[allow(dead_code)]
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer_tx: mpsc::SyncSender<Vec<u8>>,
    killer: Arc<Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
}

#[derive(Default)]
pub struct LocalSessionManager {
    sessions: Arc<Mutex<HashMap<String, LocalSession>>>,
}

impl LocalSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(&self, app: AppHandle, cols: u32, rows: u32) -> Result<String, String> {
        let session_id = Uuid::new_v4().to_string();
        info!(session_id, "Starting local terminal session");

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {}", e))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.arg("-l");
        cmd.cwd(&home);
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;
        let (writer_tx, writer_rx) = mpsc::sync_channel(LOCAL_WRITE_QUEUE_CAPACITY);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

        let killer: Arc<Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>> =
            Arc::new(Mutex::new(child.clone_killer()));

        let sid = session_id.clone();
        let sessions = self.sessions.clone();
        let app_clone = app.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        info!(session_id = %sid, "PTY reader EOF");
                        let _ = app_clone.emit(
                            "local_output",
                            LocalOutputEvent {
                                session_id: sid.clone(),
                                output: Vec::new(),
                                closed: true,
                            },
                        );
                        break;
                    }
                    Ok(n) => {
                        let output = buf[..n].to_vec();
                        if let Err(e) = app_clone.emit(
                            "local_output",
                            LocalOutputEvent {
                                session_id: sid.clone(),
                                output,
                                closed: false,
                            },
                        ) {
                            warn!(session_id = %sid, "Failed to emit local_output: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!(session_id = %sid, "PTY read error: {}", e);
                        let _ = app_clone.emit(
                            "local_output",
                            LocalOutputEvent {
                                session_id: sid.clone(),
                                output: Vec::new(),
                                closed: true,
                            },
                        );
                        break;
                    }
                }
            }
            let _ = sessions.blocking_lock().remove(&sid);
        });

        let writer_sid = session_id.clone();
        std::thread::spawn(move || run_local_write_loop(writer_sid, writer, writer_rx));

        self.sessions.lock().await.insert(
            session_id.clone(),
            LocalSession {
                child,
                master: pair.master,
                writer_tx,
                killer,
            },
        );

        info!(session_id, "Local terminal session started");
        Ok(session_id)
    }

    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        if data.len() > LOCAL_WRITE_MAX_BYTES {
            return Err("Local terminal input payload is too large".to_string());
        }

        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        session
            .writer_tx
            .try_send(data)
            .map_err(map_local_write_queue_error)?;

        Ok(())
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        session
            .master
            .resize(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to resize PTY: {}", e))?;

        info!(session_id, cols, rows, "Resized local terminal PTY");
        Ok(())
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.remove(session_id) {
            info!(session_id, "Disconnecting local terminal session");
            let mut killer = session.killer.lock().await;
            let _ = killer.kill();
        }
        Ok(())
    }
}

fn run_local_write_loop(
    session_id: String,
    mut writer: Box<dyn Write + Send>,
    writer_rx: mpsc::Receiver<Vec<u8>>,
) {
    while let Ok(data) = writer_rx.recv() {
        if let Err(error) = writer.write_all(&data).and_then(|()| writer.flush()) {
            warn!(session_id, "Local PTY write loop failed: {error}");
            break;
        }
    }
}

fn map_local_write_queue_error(error: mpsc::TrySendError<Vec<u8>>) -> String {
    match error {
        mpsc::TrySendError::Full(_) => "Local terminal input queue is full".to_string(),
        mpsc::TrySendError::Disconnected(_) => {
            "Local terminal write loop is no longer available".to_string()
        }
    }
}

#[derive(Clone, Serialize)]
pub struct LocalOutputEvent {
    pub session_id: String,
    pub output: Vec<u8>,
    pub closed: bool,
}

#[cfg(test)]
mod tests {
    use super::LocalSessionManager;

    #[test]
    fn local_session_manager_is_constructible() {
        let _ = LocalSessionManager::new();
    }
}
