use std::time::{Duration, Instant};

use russh_sftp::client::SftpSession as RusshSftpSession;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::{info, warn};

use super::transfer::{
    speed_bps, SFTP_MAX_CONCURRENT_WRITES, SFTP_MAX_PACKET_LEN, SFTP_REQUEST_TIMEOUT_SECS,
};
use super::{SftpError, TransferCancellation, TransferDirection};

const TRANSFER_LOG_INTERVAL: Duration = Duration::from_secs(2);
const TRANSFER_LOG_BYTES: u64 = 1024 * 1024;
const WRITE_STALL_WARN_AFTER: Duration = Duration::from_secs(2);
const FAILED_UPLOAD_CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);

fn sftp_request_timeout() -> Duration {
    Duration::from_secs(SFTP_REQUEST_TIMEOUT_SECS)
}

fn sftp_timeout_error(operation: &str, bytes_transferred: u64) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!(
            "SFTP {operation} timed out after {SFTP_REQUEST_TIMEOUT_SECS}s waiting for remote ACKs at {bytes_transferred} bytes"
        ),
    )
}

fn sftp_cancelled_error(operation: &str, cancel: &TransferCancellation) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        cancel
            .reason()
            .unwrap_or_else(|| format!("SFTP {operation} cancelled")),
    )
}

async fn wait_for_cancellation(cancel: &TransferCancellation) {
    while !cancel.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

pub(super) struct TransferLogger {
    transfer_id: String,
    total_bytes: u64,
    direction: TransferDirection,
    started_at: Instant,
    last_log_at: Instant,
    last_log_bytes: u64,
}

impl TransferLogger {
    pub(super) fn new(transfer_id: String, total_bytes: u64, direction: TransferDirection) -> Self {
        let now = Instant::now();
        Self {
            transfer_id,
            total_bytes,
            direction,
            started_at: now,
            last_log_at: now,
            last_log_bytes: 0,
        }
    }

    pub(super) fn log_started(&self, chunk_size: usize) {
        info!(
            transfer_id = %self.transfer_id,
            direction = ?self.direction,
            total_bytes = self.total_bytes,
            chunk_size,
            max_concurrent_writes = SFTP_MAX_CONCURRENT_WRITES,
            max_packet_len = SFTP_MAX_PACKET_LEN,
            request_timeout_secs = SFTP_REQUEST_TIMEOUT_SECS,
            "SFTP transfer started"
        );
    }

    pub(super) fn maybe_log_progress(&mut self, bytes_transferred: u64) {
        let now = Instant::now();
        let byte_delta = bytes_transferred.saturating_sub(self.last_log_bytes);
        let elapsed_since_log = now.duration_since(self.last_log_at);
        let complete = bytes_transferred == self.total_bytes;

        if !complete && byte_delta < TRANSFER_LOG_BYTES && elapsed_since_log < TRANSFER_LOG_INTERVAL
        {
            return;
        }

        info!(
            transfer_id = %self.transfer_id,
            direction = ?self.direction,
            bytes_transferred,
            total_bytes = self.total_bytes,
            elapsed_ms = now.duration_since(self.started_at).as_millis(),
            speed_bps = speed_bps(bytes_transferred, now.duration_since(self.started_at)),
            "SFTP transfer progress"
        );
        self.last_log_at = now;
        self.last_log_bytes = bytes_transferred;
    }

    pub(super) fn log_finished(&self, bytes_transferred: u64) {
        info!(
            transfer_id = %self.transfer_id,
            direction = ?self.direction,
            bytes_transferred,
            total_bytes = self.total_bytes,
            elapsed_ms = Instant::now().duration_since(self.started_at).as_millis(),
            "SFTP transfer finished"
        );
    }

    pub(super) fn log_failed(&self, error: &SftpError) {
        warn!(
            transfer_id = %self.transfer_id,
            direction = ?self.direction,
            total_bytes = self.total_bytes,
            elapsed_ms = Instant::now().duration_since(self.started_at).as_millis(),
            error = %error,
            "SFTP transfer failed"
        );
    }
}

pub(super) async fn write_all_with_stall_warning<W>(
    writer: &mut W,
    buffer: &[u8],
    logger: &TransferLogger,
    cancel: &TransferCancellation,
    bytes_transferred: u64,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let started_at = Instant::now();
    let write = writer.write_all(buffer);
    tokio::pin!(write);
    let timeout = tokio::time::sleep(sftp_request_timeout());
    tokio::pin!(timeout);
    let mut warned = false;

    loop {
        tokio::select! {
            _ = wait_for_cancellation(cancel) => {
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    chunk_bytes = buffer.len(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    reason = ?cancel.reason(),
                    "SFTP write cancelled"
                );
                return Err(sftp_cancelled_error("write", cancel));
            }
            _ = &mut timeout => {
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    chunk_bytes = buffer.len(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    max_concurrent_writes = SFTP_MAX_CONCURRENT_WRITES,
                    "SFTP write timed out waiting for ACK/window"
                );
                return Err(sftp_timeout_error("write", bytes_transferred));
            }
            result = &mut write => {
                if warned {
                    info!(
                        transfer_id = %logger.transfer_id,
                        direction = ?logger.direction,
                        bytes_transferred,
                        chunk_bytes = buffer.len(),
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "SFTP write resumed after waiting for ACK/window"
                    );
                }
                return result;
            }
            _ = tokio::time::sleep(WRITE_STALL_WARN_AFTER), if !warned => {
                warned = true;
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    chunk_bytes = buffer.len(),
                    stall_ms = WRITE_STALL_WARN_AFTER.as_millis(),
                    max_concurrent_writes = SFTP_MAX_CONCURRENT_WRITES,
            "SFTP write is waiting for ACK"
                );
            }
        }
    }
}

pub(super) async fn flush_with_stall_warning<W>(
    writer: &mut W,
    logger: &TransferLogger,
    cancel: &TransferCancellation,
    bytes_transferred: u64,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let started_at = Instant::now();
    info!(
        transfer_id = %logger.transfer_id,
        direction = ?logger.direction,
        bytes_transferred,
        "SFTP flush started"
    );

    let flush = writer.flush();
    tokio::pin!(flush);
    let timeout = tokio::time::sleep(sftp_request_timeout());
    tokio::pin!(timeout);
    let mut warned = false;

    loop {
        tokio::select! {
            _ = wait_for_cancellation(cancel) => {
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    reason = ?cancel.reason(),
                    "SFTP flush cancelled"
                );
                return Err(sftp_cancelled_error("flush", cancel));
            }
            _ = &mut timeout => {
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "SFTP flush timed out waiting for outstanding ACKs"
                );
                return Err(sftp_timeout_error("flush", bytes_transferred));
            }
            result = &mut flush => {
                info!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "SFTP flush finished"
                );
                return result;
            }
            _ = tokio::time::sleep(WRITE_STALL_WARN_AFTER), if !warned => {
                warned = true;
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    stall_ms = WRITE_STALL_WARN_AFTER.as_millis(),
                    "SFTP flush is waiting for outstanding ACKs"
                );
            }
        }
    }
}

pub(super) async fn shutdown_with_stall_warning<W>(
    writer: &mut W,
    logger: &TransferLogger,
    cancel: &TransferCancellation,
    bytes_transferred: u64,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let started_at = Instant::now();
    info!(
        transfer_id = %logger.transfer_id,
        direction = ?logger.direction,
        bytes_transferred,
        "SFTP shutdown started"
    );

    let shutdown = writer.shutdown();
    tokio::pin!(shutdown);
    let timeout = tokio::time::sleep(sftp_request_timeout());
    tokio::pin!(timeout);
    let mut warned = false;

    loop {
        tokio::select! {
            _ = wait_for_cancellation(cancel) => {
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    reason = ?cancel.reason(),
                    "SFTP shutdown cancelled"
                );
                return Err(sftp_cancelled_error("shutdown", cancel));
            }
            _ = &mut timeout => {
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "SFTP shutdown timed out waiting for close ACK"
                );
                return Err(sftp_timeout_error("shutdown", bytes_transferred));
            }
            result = &mut shutdown => {
                info!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "SFTP shutdown finished"
                );
                return result;
            }
            _ = tokio::time::sleep(WRITE_STALL_WARN_AFTER), if !warned => {
                warned = true;
                warn!(
                    transfer_id = %logger.transfer_id,
                    direction = ?logger.direction,
                    bytes_transferred,
                    stall_ms = WRITE_STALL_WARN_AFTER.as_millis(),
                    "SFTP shutdown is waiting for close ACK"
                );
            }
        }
    }
}

pub(super) async fn cleanup_failed_upload(
    session: &RusshSftpSession,
    remote_path: &str,
    logger: &TransferLogger,
) {
    info!(
        transfer_id = %logger.transfer_id,
        direction = ?logger.direction,
        remote_path,
        timeout_ms = FAILED_UPLOAD_CLEANUP_TIMEOUT.as_millis(),
        "SFTP failed upload cleanup started"
    );

    match tokio::time::timeout(
        FAILED_UPLOAD_CLEANUP_TIMEOUT,
        session.remove_file(remote_path),
    )
    .await
    {
        Ok(Ok(())) => info!(
            transfer_id = %logger.transfer_id,
            direction = ?logger.direction,
            remote_path,
            "SFTP failed upload cleanup finished"
        ),
        Ok(Err(error)) => warn!(
            transfer_id = %logger.transfer_id,
            direction = ?logger.direction,
            remote_path,
            error = %error,
            "SFTP failed upload cleanup failed"
        ),
        Err(_) => warn!(
            transfer_id = %logger.transfer_id,
            direction = ?logger.direction,
            remote_path,
            timeout_ms = FAILED_UPLOAD_CLEANUP_TIMEOUT.as_millis(),
            "SFTP failed upload cleanup timed out"
        ),
    }
}
