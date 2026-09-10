use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{mpsc, Arc};
use tokio::sync::{oneshot, Mutex};
use tracing::info;
use uuid::Uuid;

use super::terminal_output::{OutputSession, TerminalOutput, OUTPUT_CHUNK_BYTES};

const LOCAL_WRITE_QUEUE_CAPACITY: usize = 16;
const LOCAL_WRITE_MAX_BYTES: usize = 1024 * 1024;

pub struct LocalSession {
    #[allow(dead_code)]
    child: Box<dyn Child + Send + Sync>,
    writer_tx: mpsc::SyncSender<LocalWriteRequest>,
    killer: Arc<Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
}

struct LocalWriteRequest {
    operation: LocalOperation,
    completion: oneshot::Sender<Result<(), String>>,
}

enum LocalOperation {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
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
        output: &TerminalOutput,
        cols: u32,
        rows: u32,
    ) -> Result<String, String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.arg("-l");
        cmd.cwd(&home);
        cmd.env("TERM", "xterm-256color");
        self.spawn_command(output, cols, rows, cmd).await
    }

    async fn spawn_command(
        &self,
        output: &TerminalOutput,
        cols: u32,
        rows: u32,
        cmd: CommandBuilder,
    ) -> Result<String, String> {
        let session_id = Uuid::new_v4().to_string();
        let output = output.open(session_id.clone())?;
        info!(session_id, "Starting local terminal session");

        let pair = native_pty_system()
            .openpty(PtySize {
                rows: rows as u16,
                cols: cols as u16,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open PTY: {e}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {e}"))?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {e}"))?;
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn shell: {e}"))?;
        // Keeping the parent's slave open prevents EOF after the shell exits.
        drop(pair.slave);
        let (writer_tx, writer_rx) = mpsc::sync_channel(LOCAL_WRITE_QUEUE_CAPACITY);
        let killer: Arc<Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>> =
            Arc::new(Mutex::new(child.clone_killer()));
        self.sessions.lock().await.insert(
            session_id.clone(),
            LocalSession {
                child,
                writer_tx,
                killer: killer.clone(),
            },
        );

        let stop_output = output.clone();
        tokio::spawn(async move {
            stop_output.stopped().await;
            let _ = killer.lock().await.kill();
        });
        let writer_output = output.clone();
        std::thread::spawn(move || {
            run_local_write_loop(writer, pair.master, writer_rx, writer_output)
        });

        let sid = session_id.clone();
        let sessions = self.sessions.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; OUTPUT_CHUNK_BYTES];
            let error = loop {
                match reader.read(&mut buf) {
                    Ok(0) => break None,
                    Ok(n) => {
                        if let Err(error) = tauri::async_runtime::block_on(output.send(&buf[..n])) {
                            break Some(error);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    // Unix PTYs report EIO when the final slave closes.
                    #[cfg(target_os = "linux")]
                    Err(error) if error.raw_os_error() == Some(5) => break None,
                    Err(error) => break Some(format!("PTY read failed: {error}")),
                }
            };
            let _ = output.close(error);
            sessions.blocking_lock().remove(&sid);
        });

        Ok(session_id)
    }

    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        if data.len() > LOCAL_WRITE_MAX_BYTES {
            return Err("Local terminal input payload is too large".to_string());
        }
        self.request(session_id, LocalOperation::Data(data)).await
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<(), String> {
        self.request(session_id, LocalOperation::Resize { cols, rows })
            .await
    }

    async fn request(&self, session_id: &str, operation: LocalOperation) -> Result<(), String> {
        let (completion, result) = oneshot::channel();
        {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| format!("Session not found: {session_id}"))?;
            session
                .writer_tx
                .try_send(LocalWriteRequest {
                    operation,
                    completion,
                })
                .map_err(|error| match error {
                    mpsc::TrySendError::Full(_) => "Local terminal input queue is full".to_string(),
                    mpsc::TrySendError::Disconnected(_) => {
                        "Local terminal write loop is no longer available".to_string()
                    }
                })?;
        }
        result
            .await
            .map_err(|_| "Local terminal write loop stopped before completing input".to_string())?
    }

    pub async fn disconnect(&self, session_id: &str) -> Result<(), String> {
        let session = self.sessions.lock().await.remove(session_id);
        if let Some(session) = session {
            session
                .killer
                .lock()
                .await
                .kill()
                .map_err(|e| format!("Failed to stop local terminal: {e}"))?;
        }
        Ok(())
    }
}

fn run_local_write_loop(
    mut writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    writer_rx: mpsc::Receiver<LocalWriteRequest>,
    output: Arc<OutputSession>,
) {
    while let Ok(request) = writer_rx.recv() {
        let result = match request.operation {
            LocalOperation::Data(data) => write_local_data(writer.as_mut(), &data),
            LocalOperation::Resize { cols, rows } => master
                .resize(PtySize {
                    rows: rows as u16,
                    cols: cols as u16,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| format!("Failed to resize PTY: {e}")),
        };
        if let Err(error) = &result {
            output.fail(error.clone());
        }
        let failed = result.is_err();
        let _ = request.completion.send(result);
        if failed {
            break;
        }
    }
}

fn write_local_data(writer: &mut dyn Write, data: &[u8]) -> Result<(), String> {
    writer
        .write_all(data)
        .and_then(|()| writer.flush())
        .map_err(|e| format!("Local PTY write failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_write_reports_flush_failure() {
        struct FlushFailure;
        impl Write for FlushFailure {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }
        assert!(write_local_data(&mut FlushFailure, b"key").is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn local_pty_streams_binary_output_and_closes_after_final_bytes() {
        use std::time::Duration;
        use tauri::ipc::{Channel, InvokeResponseBody};

        let output = TerminalOutput::default();
        let (frames, mut received) = tokio::sync::mpsc::unbounded_channel();
        output
            .subscribe(Channel::new(move |body| {
                let InvokeResponseBody::Raw(bytes) = body else {
                    panic!("expected raw output")
                };
                frames.send(bytes).expect("PTY receiver alive");
                Ok(())
            }))
            .expect("subscribe");
        let manager = LocalSessionManager::new();
        let home = tempfile::tempdir().expect("isolated PTY home");
        let mut command = CommandBuilder::new("/bin/sh");
        command.arg("-s");
        command.cwd(home.path());
        command.env("HOME", home.path());
        command.env("ENV", "/dev/null");
        command.env("HISTFILE", "/dev/null");
        let id = manager
            .spawn_command(&output, 80, 24, command)
            .await
            .expect("spawn PTY");
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            manager
                .write(
                    &id,
                    b"printf '%s%s\\n' 'noverterm-pty-' 'verified'; exit\n".to_vec(),
                )
                .await
                .expect("write shell command");
            let mut data = Vec::new();
            loop {
                let frame = received.recv().await.expect("PTY output");
                assert_eq!(&frame[1..37], id.as_bytes());
                match frame[0] {
                    0 => {
                        data.extend_from_slice(&frame[37..]);
                        output.ack(&id, (frame.len() - 37) as u32).expect("ack");
                        assert!(data.len() <= 256 * 1024, "unexpected shell startup output");
                    }
                    1 => break,
                    2 => panic!("PTY error: {}", String::from_utf8_lossy(&frame[37..])),
                    _ => panic!("invalid frame kind"),
                }
            }
            assert!(data
                .windows(b"noverterm-pty-verified".len())
                .any(|window| window == b"noverterm-pty-verified"));
        })
        .await;
        let _ = manager.disconnect(&id).await;
        result.expect("PTY close before timeout");
    }
}
