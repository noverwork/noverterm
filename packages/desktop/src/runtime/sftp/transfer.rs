use std::time::{Duration, Instant};

use russh_sftp::client::{Config as RusshSftpConfig, SftpSession as RusshSftpSession};
use russh_sftp::protocol::OpenFlags;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::io::{
    cleanup_failed_upload, flush_with_stall_warning, shutdown_with_stall_warning,
    write_all_with_stall_warning, TransferLogger,
};
use super::types::classify_sftp_error;
use super::{SftpError, TransferCancellation, TransferDirection, TransferProgress};

// One write in flight caps throughput at UPLOAD_CHUNK_SIZE per round trip, so a
// 16ms link tops out near 1 MB/s. PuTTY (and FileZilla's fzsftp, derived from it)
// keeps writes flowing instead of waiting for each ACK; these values keep about
// 1 MiB outstanding, matching PuTTY's transfer window.
pub(super) const SFTP_MAX_CONCURRENT_WRITES: usize = 8;
pub(super) const SFTP_MAX_PACKET_LEN: u32 = 256 * 1024;
pub(super) const SFTP_REQUEST_TIMEOUT_SECS: u64 = 30;
pub(super) const UPLOAD_CHUNK_SIZE: usize = 128 * 1024;
/// How many bytes an upload may hand to russh-sftp before draining the ACKs.
/// `File::poll_write` returns as soon as a request is queued, so this is also
/// the granularity at which upload progress can be reported truthfully.
const UPLOAD_ACK_WINDOW_BYTES: u64 = (SFTP_MAX_CONCURRENT_WRITES * UPLOAD_CHUNK_SIZE) as u64;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) fn sftp_client_config() -> RusshSftpConfig {
    RusshSftpConfig {
        max_packet_len: SFTP_MAX_PACKET_LEN,
        max_concurrent_writes: SFTP_MAX_CONCURRENT_WRITES,
        request_timeout_secs: SFTP_REQUEST_TIMEOUT_SECS,
    }
}

pub(super) struct ProgressEmitter {
    transfer_id: String,
    total_bytes: u64,
    direction: TransferDirection,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    start: Instant,
    last_emit_at: Instant,
    last_emit_bytes: u64,
}

impl ProgressEmitter {
    pub(super) fn new(
        transfer_id: String,
        total_bytes: u64,
        direction: TransferDirection,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
    ) -> Self {
        let now = Instant::now();
        Self {
            transfer_id,
            total_bytes,
            direction,
            progress_tx,
            start: now,
            last_emit_at: now,
            last_emit_bytes: 0,
        }
    }

    pub(super) fn maybe_emit(&mut self, bytes_transferred: u64) -> Option<TransferProgress> {
        self.maybe_emit_at(bytes_transferred, Instant::now())
    }

    pub(super) fn emit_final(&mut self, bytes_transferred: u64) -> Option<TransferProgress> {
        if bytes_transferred == self.last_emit_bytes {
            return None;
        }

        self.emit(bytes_transferred, Instant::now())
    }

    fn maybe_emit_at(&mut self, bytes_transferred: u64, now: Instant) -> Option<TransferProgress> {
        if bytes_transferred == self.last_emit_bytes {
            return None;
        }

        let enough_time = now.duration_since(self.last_emit_at) >= PROGRESS_INTERVAL;
        let complete = bytes_transferred == self.total_bytes;

        if complete || enough_time {
            return self.emit(bytes_transferred, now);
        }

        None
    }

    fn emit(&mut self, bytes_transferred: u64, now: Instant) -> Option<TransferProgress> {
        let progress = TransferProgress {
            transfer_id: self.transfer_id.clone(),
            bytes_transferred,
            total_bytes: self.total_bytes,
            speed_bps: speed_bps(bytes_transferred, now.duration_since(self.start)),
            direction: self.direction,
        };

        if let Some(progress_tx) = &self.progress_tx {
            let _ = progress_tx.send(progress.clone());
        }

        self.last_emit_at = now;
        self.last_emit_bytes = bytes_transferred;
        Some(progress)
    }
}

pub(super) fn speed_bps(bytes_transferred: u64, elapsed: Duration) -> u64 {
    let elapsed_secs = elapsed.as_secs_f64();
    if elapsed_secs <= f64::EPSILON {
        return 0;
    }

    (bytes_transferred as f64 / elapsed_secs) as u64
}

pub async fn upload_sftp(
    session: &RusshSftpSession,
    local_path: &str,
    remote_path: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let mut local_file = tokio::fs::File::open(local_path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
    let total_bytes = local_file
        .metadata()
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?
        .len();
    let mut remote_file = session
        .open_with_flags(
            remote_path,
            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
        )
        .await
        .map_err(|error| classify_sftp_error("write", remote_path, error))?;

    let mut progress = ProgressEmitter::new(
        transfer_id.clone(),
        total_bytes,
        TransferDirection::Upload,
        progress_tx,
    );
    let mut transfer_log = TransferLogger::new(transfer_id, total_bytes, TransferDirection::Upload);
    transfer_log.log_started(UPLOAD_CHUNK_SIZE);
    let mut buffer = vec![0; UPLOAD_CHUNK_SIZE];
    // Bytes handed to russh-sftp versus bytes the server has acknowledged.
    let mut queued_bytes = 0u64;
    let mut bytes_transferred = 0u64;

    let result = async {
        loop {
            if cancel.is_cancelled() {
                return Err(SftpError::OperationFailed(
                    cancel
                        .reason()
                        .unwrap_or_else(|| "upload cancelled".to_string()),
                ));
            }

            let read_count = local_file
                .read(&mut buffer)
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            if read_count == 0 {
                break;
            }

            write_all_with_stall_warning(
                &mut remote_file,
                &buffer[..read_count],
                &transfer_log,
                &cancel,
                queued_bytes,
            )
            .await
            .map_err(|error| classify_sftp_error("write", remote_path, error))?;
            queued_bytes += read_count as u64;

            // Only a flush proves the server stored the queued writes, so progress
            // stays put until then; reporting queued bytes would race ahead of the
            // transfer and finish the status bar while data is still in flight.
            if queued_bytes - bytes_transferred >= UPLOAD_ACK_WINDOW_BYTES {
                flush_with_stall_warning(
                    &mut remote_file,
                    &transfer_log,
                    &cancel,
                    bytes_transferred,
                )
                .await
                .map_err(|error| classify_sftp_error("write", remote_path, error))?;
                bytes_transferred = queued_bytes;
                progress.maybe_emit(bytes_transferred);
                transfer_log.maybe_log_progress(bytes_transferred);
            }
        }

        flush_with_stall_warning(&mut remote_file, &transfer_log, &cancel, bytes_transferred)
            .await
            .map_err(|error| classify_sftp_error("write", remote_path, error))?;
        bytes_transferred = queued_bytes;
        shutdown_with_stall_warning(&mut remote_file, &transfer_log, &cancel, bytes_transferred)
            .await
            .map_err(|error| classify_sftp_error("write", remote_path, error))?;

        progress.emit_final(bytes_transferred);
        transfer_log.log_finished(bytes_transferred);
        Ok(bytes_transferred)
    }
    .await;

    if result.is_err() {
        if let Err(error) = &result {
            transfer_log.log_failed(error);
        }
        cleanup_failed_upload(session, remote_path, &transfer_log).await;
    }

    result
}

pub async fn download_sftp(
    session: &RusshSftpSession,
    remote_path: &str,
    local_path: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let total_bytes = session
        .metadata(remote_path)
        .await
        .map_err(|error| classify_sftp_error("read", remote_path, error))?
        .len();
    let mut remote_file = session
        .open_with_flags(remote_path, OpenFlags::READ)
        .await
        .map_err(|error| classify_sftp_error("read", remote_path, error))?;
    let mut local_file = tokio::fs::File::create(local_path)
        .await
        .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
    let mut progress = ProgressEmitter::new(
        transfer_id,
        total_bytes,
        TransferDirection::Download,
        progress_tx,
    );
    let mut buffer = vec![0; UPLOAD_CHUNK_SIZE];
    let mut bytes_transferred = 0;

    let result = async {
        loop {
            if cancel.is_cancelled() {
                return Err(SftpError::OperationFailed("download cancelled".to_string()));
            }

            let read_count = remote_file
                .read(&mut buffer)
                .await
                .map_err(|error| classify_sftp_error("read", remote_path, error))?;
            if read_count == 0 {
                break;
            }

            local_file
                .write_all(&buffer[..read_count])
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            bytes_transferred += read_count as u64;
            progress.maybe_emit(bytes_transferred);
        }

        local_file
            .flush()
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        progress.emit_final(bytes_transferred);
        Ok(bytes_transferred)
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(local_path).await;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_emitter_throttles_fast_chunks_until_complete() {
        let total_bytes = (UPLOAD_CHUNK_SIZE * 100) as u64;
        let mut progress = ProgressEmitter::new(
            "fast-transfer".to_string(),
            total_bytes,
            TransferDirection::Upload,
            None,
        );
        let start = progress.last_emit_at;
        let mut events = Vec::new();

        for chunk_index in 1..=100 {
            let bytes_transferred = (UPLOAD_CHUNK_SIZE * chunk_index) as u64;
            let now = start + Duration::from_micros((chunk_index * 500) as u64);
            if let Some(event) = progress.maybe_emit_at(bytes_transferred, now) {
                events.push(event);
            }
        }

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].transfer_id, "fast-transfer");
        assert_eq!(events[0].bytes_transferred, total_bytes);
        assert_eq!(events[0].direction, TransferDirection::Upload);
    }
}
