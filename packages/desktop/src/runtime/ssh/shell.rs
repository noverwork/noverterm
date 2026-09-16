use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use russh::client::Msg;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf};
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::warn;

use crate::runtime::terminal_output::{OutputSession, OUTPUT_CHUNK_BYTES};

use super::{
    remove_session_and_stop_port_forwards, SshSession, SshSessionManager, SshWriteOperation,
    SshWriteRequest,
};

const SSH_OUTPUT_FLUSH_INTERVAL: Duration = Duration::from_millis(2);
pub(super) const SSH_WRITE_QUEUE_CAPACITY: usize = 16;
const SSH_WRITE_MAX_BYTES: usize = 1024 * 1024;

impl SshSessionManager {
    pub async fn write(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        if data.len() > SSH_WRITE_MAX_BYTES {
            return Err("SSH terminal input payload is too large".to_string());
        }
        self.shell_request(session_id, SshWriteOperation::Data(data))
            .await
    }

    async fn shell_request(
        &self,
        session_id: &str,
        operation: SshWriteOperation,
    ) -> Result<(), String> {
        let (completion, result) = oneshot::channel();
        {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| format!("Session not found: {session_id}"))?;
            let write_tx = session
                .write_tx
                .as_ref()
                .ok_or("No shell channel available for this session")?;
            write_tx
                .try_send(SshWriteRequest {
                    operation,
                    completion,
                })
                .map_err(map_ssh_write_queue_error)?;
        }
        result
            .await
            .map_err(|_| "SSH write loop stopped before completing input".to_string())?
    }

    pub async fn resize(&self, session_id: &str, cols: u32, rows: u32) -> Result<(), String> {
        self.shell_request(session_id, SshWriteOperation::Resize { cols, rows })
            .await
    }
}

struct OutputBatch {
    bytes: Vec<u8>,
    last_flush: Option<tokio::time::Instant>,
}

impl OutputBatch {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(OUTPUT_CHUNK_BYTES),
            last_flush: None,
        }
    }

    fn deadline(&self) -> tokio::time::Instant {
        self.last_flush.unwrap_or_else(tokio::time::Instant::now) + SSH_OUTPUT_FLUSH_INTERVAL
    }

    async fn push(&mut self, mut data: &[u8], output: &OutputSession) -> Result<(), String> {
        // Idle output is never held for a batching timer.
        if self.bytes.is_empty()
            && self
                .last_flush
                .is_none_or(|last| last.elapsed() >= SSH_OUTPUT_FLUSH_INTERVAL)
        {
            output.send(data).await?;
            self.last_flush = Some(tokio::time::Instant::now());
            return Ok(());
        }
        while !data.is_empty() {
            let take = data.len().min(OUTPUT_CHUNK_BYTES - self.bytes.len());
            self.bytes.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.bytes.len() == OUTPUT_CHUNK_BYTES {
                self.flush(output).await?;
            }
        }
        Ok(())
    }

    async fn flush(&mut self, output: &OutputSession) -> Result<(), String> {
        if !self.bytes.is_empty() {
            output.send(&self.bytes).await?;
            self.bytes.clear();
            self.last_flush = Some(tokio::time::Instant::now());
        }
        Ok(())
    }
}

pub(super) async fn read_loop(
    mut channel_read: ChannelReadHalf,
    session_id: String,
    app: AppHandle,
    sessions: Arc<Mutex<HashMap<String, SshSession>>>,
    output: Arc<OutputSession>,
) {
    let mut batch = OutputBatch::new();
    let error = loop {
        tokio::select! {
            biased;
            _ = output.stopped() => break None,
            _ = tokio::time::sleep_until(batch.deadline()), if !batch.bytes.is_empty() => {
                if let Err(error) = batch.flush(&output).await { break Some(error); }
            }
            message = channel_read.wait() => {
                match message {
                    Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                        if let Err(error) = batch.push(&data, &output).await { break Some(error); }
                    }
                    Some(ChannelMsg::Eof | ChannelMsg::Close) | None => break None,
                    Some(ChannelMsg::Failure) => break Some("SSH channel request failed".to_string()),
                    Some(_) => {}
                }
            }
        }
    };
    let flush_error = batch.flush(&output).await.err();
    let error = error.or(flush_error);
    if let Err(close_error) = output.close(error.clone()) {
        warn!(session_id, %close_error, "Failed to close terminal output");
    }
    remove_session_and_stop_port_forwards(sessions, &session_id, &app, error, true).await;
}

pub(super) async fn run_shell_write_loop(
    channel_write: ChannelWriteHalf<Msg>,
    mut write_rx: mpsc::Receiver<SshWriteRequest>,
    output: Arc<OutputSession>,
) {
    while let Some(request) = tokio::select! {
        _ = output.stopped() => None,
        request = write_rx.recv() => request,
    } {
        let result = tokio::select! {
            _ = output.stopped() => Err("SSH session closed before completing input".to_string()),
            result = async {
                match request.operation {
                    SshWriteOperation::Data(data) => channel_write.data(&data[..]).await,
                    SshWriteOperation::Resize { cols, rows } => channel_write.window_change(cols, rows, 0, 0).await,
                }
            } => result.map_err(|e| format!("SSH terminal write failed: {e}")),
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

fn map_ssh_write_queue_error(error: mpsc::error::TrySendError<SshWriteRequest>) -> String {
    match error {
        mpsc::error::TrySendError::Full(_) => "SSH terminal input queue is full".to_string(),
        mpsc::error::TrySendError::Closed(_) => "SSH write loop is no longer available".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn idle_output_is_immediate_and_burst_flush_preserves_final_bytes() {
        use crate::runtime::terminal_output::{TerminalOutput, OUTPUT_CHUNK_BYTES};
        use std::future::Future;
        use std::task::Poll;
        use tauri::ipc::{Channel, InvokeResponseBody};

        let frames = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received = frames.clone();
        let transport = TerminalOutput::default();
        transport
            .subscribe(Channel::new(move |body| {
                let InvokeResponseBody::Raw(bytes) = body else {
                    panic!("expected raw bytes")
                };
                received.lock().expect("frames").push(bytes);
                Ok(())
            }))
            .expect("subscribe");
        let output = transport
            .open(uuid::Uuid::new_v4().to_string())
            .expect("open");
        let mut batch = OutputBatch::new();
        {
            let first = batch.push(b"prompt", &output);
            tokio::pin!(first);
            assert!(matches!(
                std::future::poll_fn(|cx| Poll::Ready(first.as_mut().poll(cx))).await,
                Poll::Ready(Ok(()))
            ));
        }
        assert_eq!(&frames.lock().expect("frames")[0][37..], b"prompt");
        batch
            .push(&vec![7; OUTPUT_CHUNK_BYTES + 3], &output)
            .await
            .expect("burst");
        batch.flush(&output).await.expect("flush EOF");
        output.close(None).expect("close");
        let frames = frames.lock().expect("frames");
        assert!(frames
            .iter()
            .all(|frame| frame.len() <= 37 + OUTPUT_CHUNK_BYTES));
        let bytes: Vec<u8> = frames
            .iter()
            .filter(|frame| frame[0] == 0)
            .flat_map(|frame| frame[37..].iter().copied())
            .collect();
        assert_eq!(&bytes[..6], b"prompt");
        assert_eq!(&bytes[6..], vec![7; OUTPUT_CHUNK_BYTES + 3]);
        assert_eq!(frames.last().expect("close")[0], 1);
    }
}
