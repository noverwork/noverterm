pub mod pty;
pub mod renderer;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

pub type TerminalState = Mutex<Vec<Session>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub status: SessionStatus,
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TerminalOutput {
    pub session_id: String,
    pub data: Vec<u8>,  // Raw terminal output
    pub rows: u16,
    pub cols: u16,
}

#[tauri::command]
pub fn create_session(
    name: String,
    host: String,
    port: u16,
    username: String,
    state: State<TerminalState>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let session = Session {
        id: id.clone(),
        name,
        host,
        port,
        username,
        status: SessionStatus::Disconnected,
        rows: 24,
        cols: 80,
    };

    state.lock().unwrap().push(session);
    Ok(id)
}

#[tauri::command]
pub fn get_sessions(state: State<TerminalState>) -> Vec<Session> {
    state.lock().unwrap().clone()
}

#[tauri::command]
pub fn connect_session(session_id: String, state: State<TerminalState>) -> Result<(), String> {
    let mut sessions = state.lock().unwrap();
    if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
        session.status = SessionStatus::Connecting;
        // TODO: Start PTY and SSH connection
        session.status = SessionStatus::Connected;
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}

#[tauri::command]
pub fn disconnect_session(session_id: String, state: State<TerminalState>) -> Result<(), String> {
    let mut sessions = state.lock().unwrap();
    if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
        session.status = SessionStatus::Disconnected;
        // TODO: Close PTY
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}

#[tauri::command]
pub fn send_input(
    _session_id: String,
    _data: Vec<u8>,
    _state: State<TerminalState>,
) -> Result<(), String> {
    // TODO: Send input to PTY
    Ok(())
}

#[tauri::command]
pub fn resize_pty(session_id: String, rows: u16, cols: u16, state: State<TerminalState>) -> Result<(), String> {
    let mut sessions = state.lock().unwrap();
    if let Some(session) = sessions.iter_mut().find(|s| s.id == session_id) {
        session.rows = rows;
        session.cols = cols;
        // TODO: Resize PTY
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}
