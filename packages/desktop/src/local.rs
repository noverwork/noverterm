use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

pub struct LocalSession {
    #[allow(dead_code)]
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
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

    pub async fn connect(
        &self,
        app: AppHandle,
        cols: u32,
        rows: u32,
    ) -> Result<String, String> {
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

        let mut cmd = CommandBuilder::new_default_prog();
        cmd.env("TERM", "xterm-256color");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        let writer = pair.master.take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

        let mut reader = pair.master.try_clone_reader()
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
                        let _ = app_clone.emit("local_output", LocalOutputEvent {
                            session_id: sid.clone(),
                            output: String::new(),
                            closed: true,
                        });
                        break;
                    }
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buf[..n]).to_string();
                        if let Err(e) = app_clone.emit("local_output", LocalOutputEvent {
                            session_id: sid.clone(),
                            output,
                            closed: false,
                        }) {
                            warn!(session_id = %sid, "Failed to emit local_output: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!(session_id = %sid, "PTY read error: {}", e);
                        let _ = app_clone.emit("local_output", LocalOutputEvent {
                            session_id: sid.clone(),
                            output: String::new(),
                            closed: true,
                        });
                        break;
                    }
                }
            }
            let _ = sessions.blocking_lock().remove(&sid);
        });

        self.sessions.lock().await.insert(
            session_id.clone(),
            LocalSession {
                child,
                master: pair.master,
                writer,
                killer,
            },
        );

        info!(session_id, "Local terminal session started");
        Ok(session_id)
    }

    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

        session
            .writer
            .write_all(&data)
            .map_err(|e| format!("Failed to write: {}", e))?;
        session
            .writer
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        info!(session_id, bytes = data.len(), "Sent input to local PTY");
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

#[derive(Clone, Serialize)]
pub struct LocalOutputEvent {
    pub session_id: String,
    pub output: String,
    pub closed: bool,
}
