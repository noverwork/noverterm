use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use tauri::ipc::{Channel, InvokeResponseBody, IpcResponse};
use tokio::sync::{watch, Semaphore};

pub const OUTPUT_CHUNK_BYTES: usize = 64 * 1024;
const OUTPUT_WINDOW_BYTES: usize = 256 * 1024;

/// Raw channel payload. Specta cannot describe JavaScript's ArrayBuffer, so consumers
/// must narrow the exported unknown before decoding the binary frame.
pub struct TerminalFrame(Vec<u8>);

impl specta::Type for TerminalFrame {
    fn inline(_: &mut specta::TypeMap, _: specta::Generics) -> specta::datatype::DataType {
        specta::datatype::DataType::Unknown
    }
}

impl IpcResponse for TerminalFrame {
    fn body(self) -> tauri::Result<InvokeResponseBody> {
        Ok(InvokeResponseBody::Raw(self.0))
    }
}

#[derive(Default, Clone)]
pub struct TerminalOutput {
    inner: Arc<Mutex<Transport>>,
}

#[derive(Default)]
struct Transport {
    channel: Option<Channel<TerminalFrame>>,
    sessions: HashMap<String, Weak<OutputSession>>,
}

pub struct OutputSession {
    id: String,
    transport: Weak<Mutex<Transport>>,
    capacity: Semaphore,
    state: Mutex<OutputState>,
    stopped: watch::Sender<bool>,
}

#[derive(Default)]
struct OutputState {
    in_flight: usize,
    closed: bool,
    error: Option<String>,
}

impl TerminalOutput {
    pub fn subscribe(&self, channel: Channel<TerminalFrame>) -> Result<(), String> {
        let mut transport = self.inner.lock().map_err(|e| e.to_string())?;
        transport.cancel_sessions("Terminal output subscription replaced");
        transport.channel = Some(channel);
        Ok(())
    }

    pub fn unsubscribe(&self, channel_id: u32) -> Result<(), String> {
        let mut transport = self.inner.lock().map_err(|e| e.to_string())?;
        if transport
            .channel
            .as_ref()
            .is_some_and(|channel| channel.id() == channel_id)
        {
            transport.cancel_sessions("Terminal output subscription closed");
            transport.channel = None;
        }
        Ok(())
    }

    pub fn open(&self, id: String) -> Result<Arc<OutputSession>, String> {
        let mut transport = self.inner.lock().map_err(|e| e.to_string())?;
        if transport.channel.is_none() {
            return Err("Subscribe to terminal output before connecting".to_string());
        }
        let (stopped, _) = watch::channel(false);
        let output = Arc::new(OutputSession {
            id: id.clone(),
            transport: Arc::downgrade(&self.inner),
            capacity: Semaphore::new(OUTPUT_WINDOW_BYTES),
            state: Mutex::new(OutputState::default()),
            stopped,
        });
        transport
            .sessions
            .retain(|_, session| session.strong_count() > 0);
        transport.sessions.insert(id, Arc::downgrade(&output));
        Ok(output)
    }

    pub fn ack(&self, id: &str, bytes: u32) -> Result<(), String> {
        let transport = self.inner.lock().map_err(|e| e.to_string())?;
        let Some(output) = transport.sessions.get(id).and_then(Weak::upgrade) else {
            return Ok(());
        };
        let mut state = output.state.lock().map_err(|e| e.to_string())?;
        let bytes = bytes as usize;
        if bytes > state.in_flight {
            return Err("Terminal output acknowledgment exceeds outstanding bytes".to_string());
        }
        state.in_flight -= bytes;
        output.capacity.add_permits(bytes);
        Ok(())
    }
}

impl Transport {
    fn cancel_sessions(&mut self, reason: &str) {
        for output in self.sessions.values().filter_map(Weak::upgrade) {
            output.cancel(reason.to_string());
        }
        self.sessions.clear();
    }
}

impl OutputSession {
    /// Signal a producer failure; the reader flushes its buffer before closing.
    pub fn fail(&self, error: String) {
        if let Ok(mut state) = self.state.lock() {
            state.error.get_or_insert(error);
        }
        self.stopped.send_replace(true);
    }

    pub fn stop(&self) {
        self.stopped.send_replace(true);
    }

    fn cancel(&self, error: String) {
        self.fail(error);
        self.capacity.close();
    }

    pub async fn stopped(&self) {
        let mut stopped = self.stopped.subscribe();
        let already_stopped = *stopped.borrow_and_update();
        if !already_stopped {
            let _ = stopped.changed().await;
        }
    }

    pub async fn send(&self, data: &[u8]) -> Result<(), String> {
        for chunk in data.chunks(OUTPUT_CHUNK_BYTES) {
            // A suspended WebView may take arbitrarily long to acknowledge.
            // Subscription teardown closes the semaphore and cancels waiters.
            let permit = self
                .capacity
                .acquire_many(chunk.len() as u32)
                .await
                .map_err(|_| "Terminal output subscription closed".to_string())?;
            self.frame(0, chunk)?;
            permit.forget();
        }
        Ok(())
    }

    pub fn close(&self, error: Option<String>) -> Result<(), String> {
        if let Some(error) = error {
            self.fail(error);
        }
        self.frame(1, &[])
    }

    fn frame(&self, kind: u8, data: &[u8]) -> Result<(), String> {
        let transport = self
            .transport
            .upgrade()
            .ok_or("Terminal output transport closed")?;
        let mut transport = transport.lock().map_err(|e| e.to_string())?;
        // A replaced channel must never receive old-session frames.
        if !transport
            .sessions
            .get(&self.id)
            .is_some_and(|session| std::ptr::eq(session.as_ptr(), self))
        {
            return Err("Terminal output subscription closed".to_string());
        }
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        if state.closed {
            return if kind == 0 {
                Err("Terminal output session closed".to_string())
            } else {
                Ok(())
            };
        }
        let (kind, data) = if kind == 1 {
            state.closed = true;
            self.capacity.close();
            self.stopped.send_replace(true);
            match state.error.as_ref() {
                Some(error) => (2, error.as_bytes()),
                None => (1, data),
            }
        } else {
            state.in_flight += data.len();
            (kind, data)
        };
        let mut frame = Vec::with_capacity(37 + data.len());
        frame.push(kind);
        frame.extend_from_slice(self.id.as_bytes());
        frame.extend_from_slice(data);
        let result = transport
            .channel
            .as_ref()
            .ok_or("Terminal output subscription closed")?
            .send(TerminalFrame(frame))
            .map_err(|e| e.to_string());
        drop(state);
        if let Err(error) = &result {
            transport.cancel_sessions(error);
            transport.channel = None;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;

    type CapturedFrames = Arc<Mutex<Vec<Vec<u8>>>>;

    fn transport() -> (TerminalOutput, Arc<OutputSession>, CapturedFrames) {
        let frames = Arc::new(Mutex::new(Vec::new()));
        let received = frames.clone();
        let transport = TerminalOutput::default();
        transport
            .subscribe(Channel::new(move |body| {
                let InvokeResponseBody::Raw(bytes) = body else {
                    panic!("expected raw output")
                };
                received.lock().expect("frames").push(bytes);
                Ok(())
            }))
            .expect("subscribe");
        let output = transport
            .open(uuid::Uuid::new_v4().to_string())
            .expect("open");
        (transport, output, frames)
    }

    #[tokio::test]
    async fn first_bytes_are_ready_and_close_follows_bounded_data() {
        let (_transport, output, frames) = transport();
        let send = output.send(b"prompt");
        tokio::pin!(send);
        assert!(matches!(
            std::future::poll_fn(|cx| Poll::Ready(send.as_mut().poll(cx))).await,
            Poll::Ready(Ok(()))
        ));
        output
            .send(&vec![42; OUTPUT_CHUNK_BYTES + 3])
            .await
            .expect("send");
        output
            .close(Some("write failed".to_string()))
            .expect("close");
        let frames = frames.lock().expect("frames");
        assert_eq!(&frames[0][37..], b"prompt");
        assert_eq!(frames[1].len(), 37 + OUTPUT_CHUNK_BYTES);
        assert_eq!(&frames[2][37..], &[42; 3]);
        assert_eq!(frames[3][0], 2);
        assert_eq!(&frames[3][37..], b"write failed");
    }

    #[tokio::test]
    async fn window_blocks_until_ack_and_replacement_cancels_waiters() {
        let (transport, output, _) = transport();
        output
            .send(&vec![0; OUTPUT_WINDOW_BYTES])
            .await
            .expect("fill window");
        let send = output.send(b"x");
        tokio::pin!(send);
        assert!(matches!(
            std::future::poll_fn(|cx| Poll::Ready(send.as_mut().poll(cx))).await,
            Poll::Pending
        ));
        assert!(transport
            .ack(&output.id, OUTPUT_WINDOW_BYTES as u32 + 1)
            .is_err());
        transport.ack(&output.id, 1).expect("ack");
        send.await.expect("released");
        let blocked = output.send(b"y");
        tokio::pin!(blocked);
        assert!(matches!(
            std::future::poll_fn(|cx| Poll::Ready(blocked.as_mut().poll(cx))).await,
            Poll::Pending
        ));
        transport
            .subscribe(Channel::new(|_| Ok(())))
            .expect("replace");
        assert!(blocked.await.is_err());
        assert!(output.close(None).is_err());
    }

    #[tokio::test]
    async fn stale_unsubscribe_cannot_remove_replacement_and_loss_cancels_sessions() {
        let transport = TerminalOutput::default();
        let old = Channel::new(|_| Ok(()));
        let old_id = old.id();
        transport.subscribe(old).expect("subscribe");
        let replacement =
            Channel::new(|_| Err(tauri::Error::Io(std::io::ErrorKind::BrokenPipe.into())));
        transport.subscribe(replacement).expect("replace");
        transport.unsubscribe(old_id).expect("stale unsubscribe");
        let output = transport
            .open(uuid::Uuid::new_v4().to_string())
            .expect("replacement survives");
        assert!(output.send(b"x").await.is_err());
        output.stopped().await;
        assert!(transport.open(uuid::Uuid::new_v4().to_string()).is_err());
        assert!(output.send(b"y").await.is_err());
    }
}
