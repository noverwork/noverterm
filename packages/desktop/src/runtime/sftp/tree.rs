use std::collections::HashSet;
use std::path::PathBuf;

use super::transfer::ProgressEmitter;
use super::{
    FileType, SftpError, SftpSession, TransferCancellation, TransferDirection, TransferProgress,
};

impl SftpSession {
    /// Relative file paths that exist in both trees, so a folder transfer can
    /// tell the user which files an overwrite would clobber.
    pub async fn transfer_conflicts(
        &self,
        direction: TransferDirection,
        source_path: &str,
        target_path: &str,
    ) -> Result<Vec<String>, SftpError> {
        let (source, target) = match direction {
            TransferDirection::Upload => (
                list_local_tree(source_path).await?,
                self.list_remote_tree(target_path).await?,
            ),
            TransferDirection::Download => (
                self.list_remote_tree(source_path).await?,
                list_local_tree(target_path).await?,
            ),
            TransferDirection::Copy => (
                self.list_remote_tree(source_path).await?,
                self.list_remote_tree(target_path).await?,
            ),
        };

        Ok(common_paths(source, target))
    }

    /// `transfer_conflicts` for a copy from this session into `target`.
    pub async fn copy_conflicts(
        &self,
        source_path: &str,
        target: &SftpSession,
        target_path: &str,
    ) -> Result<Vec<String>, SftpError> {
        Ok(common_paths(
            self.list_remote_tree(source_path).await?,
            target.list_remote_tree(target_path).await?,
        ))
    }

    /// File paths under `root`, relative to it. Empty when `root` is not a directory.
    async fn list_remote_tree(&self, root: &str) -> Result<Vec<String>, SftpError> {
        match self.stat(root).await {
            Ok(entry) if entry.file_type == FileType::Dir => {}
            Ok(_) | Err(SftpError::NotFound(_)) => return Ok(Vec::new()),
            Err(error) => return Err(error),
        }

        let mut pending = vec![String::new()];
        let mut files = Vec::new();

        while let Some(relative) = pending.pop() {
            let dir = if relative.is_empty() {
                root.to_string()
            } else {
                join_remote(root, &relative)
            };

            for entry in self.list_dir(&dir).await? {
                let child = if relative.is_empty() {
                    entry.name.clone()
                } else {
                    format!("{relative}/{}", entry.name)
                };
                match entry.file_type {
                    FileType::Dir => pending.push(child),
                    FileType::File => files.push(child),
                    _ => {}
                }
            }
        }

        Ok(files)
    }
}

fn common_paths(source: Vec<String>, target: Vec<String>) -> Vec<String> {
    let target: HashSet<String> = target.into_iter().collect();
    let mut conflicts = source
        .into_iter()
        .filter(|relative| target.contains(relative))
        .collect::<Vec<_>>();
    conflicts.sort();
    conflicts
}

pub(super) async fn is_local_dir(path: &str) -> bool {
    tokio::fs::metadata(path)
        .await
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
}

/// File paths under `root`, relative to it. Empty when `root` is not a directory.
async fn list_local_tree(root: &str) -> Result<Vec<String>, SftpError> {
    if !is_local_dir(root).await {
        return Ok(Vec::new());
    }

    let mut pending = vec![String::new()];
    let mut files = Vec::new();

    while let Some(relative) = pending.pop() {
        let dir = if relative.is_empty() {
            PathBuf::from(root)
        } else {
            PathBuf::from(root).join(&relative)
        };

        let mut read_dir = tokio::fs::read_dir(&dir)
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?
        {
            let metadata = entry
                .metadata()
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let child = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };

            if metadata.is_dir() {
                pending.push(child);
            } else if metadata.is_file() {
                files.push(child);
            }
        }
    }

    Ok(files)
}

fn join_remote(parent: &str, name: &str) -> String {
    format!("{}/{}", parent.trim_end_matches('/'), name)
}

/// Recursively upload a local directory tree to `remote_root`.
///
/// ponytail: files are sent one at a time and progress is emitted per finished
/// file, so a huge file inside the tree shows no intra-file progress. Thread a
/// shared ProgressEmitter through upload_sftp if that becomes a problem.
pub(super) async fn upload_dir(
    session: &SftpSession,
    local_root: &str,
    remote_root: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let mut pending = vec![(PathBuf::from(local_root), remote_root.to_string())];
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut total_bytes = 0;

    while let Some((local_dir, remote_dir)) = pending.pop() {
        match session.mkdir(&remote_dir).await {
            Ok(()) | Err(SftpError::AlreadyExists) => {}
            Err(error) => return Err(error),
        }

        let mut read_dir = tokio::fs::read_dir(&local_dir)
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?
        {
            let metadata = entry
                .metadata()
                .await
                .map_err(|error| SftpError::OperationFailed(error.to_string()))?;
            let remote_path = join_remote(&remote_dir, &entry.file_name().to_string_lossy());
            if metadata.is_dir() {
                pending.push((entry.path(), remote_path));
            } else if metadata.is_file() {
                total_bytes += metadata.len();
                files.push((entry.path(), remote_path));
            }
        }
    }

    let mut progress = ProgressEmitter::new(
        transfer_id.clone(),
        total_bytes,
        TransferDirection::Upload,
        progress_tx,
    );
    let mut bytes_transferred = 0;

    for (local_path, remote_path) in files {
        if cancel.is_cancelled() {
            return Err(SftpError::OperationFailed(
                cancel
                    .reason()
                    .unwrap_or_else(|| "upload cancelled".to_string()),
            ));
        }

        bytes_transferred += session
            .upload_file(
                &local_path.to_string_lossy(),
                &remote_path,
                transfer_id.clone(),
                cancel.clone(),
                None,
            )
            .await?;
        progress.maybe_emit(bytes_transferred);
    }

    progress.emit_final(bytes_transferred);
    Ok(bytes_transferred)
}

/// Recursively download a remote directory tree into `local_root`.
///
/// ponytail: symlinks in the tree are skipped and progress granularity is
/// per-file, same tradeoff as upload_dir.
pub(super) async fn download_dir(
    session: &SftpSession,
    remote_root: &str,
    local_root: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let mut pending = vec![(remote_root.to_string(), PathBuf::from(local_root))];
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    let mut total_bytes = 0;

    while let Some((remote_dir, local_dir)) = pending.pop() {
        tokio::fs::create_dir_all(&local_dir)
            .await
            .map_err(|error| SftpError::OperationFailed(error.to_string()))?;

        for entry in session.list_dir(&remote_dir).await? {
            let remote_path = join_remote(&remote_dir, &entry.name);
            let local_path = local_dir.join(&entry.name);
            match entry.file_type {
                FileType::Dir => pending.push((remote_path, local_path)),
                FileType::File => {
                    total_bytes += entry.size;
                    files.push((remote_path, local_path));
                }
                _ => {}
            }
        }
    }

    let mut progress = ProgressEmitter::new(
        transfer_id.clone(),
        total_bytes,
        TransferDirection::Download,
        progress_tx,
    );
    let mut bytes_transferred = 0;

    for (remote_path, local_path) in files {
        if cancel.is_cancelled() {
            return Err(SftpError::OperationFailed("download cancelled".to_string()));
        }

        bytes_transferred += session
            .download_file(
                &remote_path,
                &local_path.to_string_lossy(),
                transfer_id.clone(),
                cancel.clone(),
                None,
            )
            .await?;
        progress.maybe_emit(bytes_transferred);
    }

    progress.emit_final(bytes_transferred);
    Ok(bytes_transferred)
}

/// Recursively copy a directory tree from `source` into `target`.
///
/// The whole source tree is listed before anything is written, so copying a
/// folder into itself on the same server cannot recurse forever.
pub(super) async fn copy_dir(
    source: &SftpSession,
    source_root: &str,
    target: &SftpSession,
    target_root: &str,
    transfer_id: String,
    cancel: TransferCancellation,
    progress_tx: Option<tokio::sync::mpsc::UnboundedSender<TransferProgress>>,
) -> Result<u64, SftpError> {
    let mut pending = vec![(source_root.to_string(), target_root.to_string())];
    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<(String, String)> = Vec::new();
    let mut total_bytes = 0;

    while let Some((source_dir, target_dir)) = pending.pop() {
        for entry in source.list_dir(&source_dir).await? {
            let source_path = join_remote(&source_dir, &entry.name);
            let target_path = join_remote(&target_dir, &entry.name);
            match entry.file_type {
                FileType::Dir => pending.push((source_path, target_path)),
                FileType::File => {
                    total_bytes += entry.size;
                    files.push((source_path, target_path));
                }
                _ => {}
            }
        }
        dirs.push(target_dir);
    }

    for dir in dirs {
        match target.mkdir(&dir).await {
            Ok(()) | Err(SftpError::AlreadyExists) => {}
            Err(error) => return Err(error),
        }
    }

    let mut progress = ProgressEmitter::new(
        transfer_id.clone(),
        total_bytes,
        TransferDirection::Copy,
        progress_tx,
    );
    let mut bytes_transferred = 0;

    for (source_path, target_path) in files {
        if cancel.is_cancelled() {
            return Err(SftpError::OperationFailed(
                cancel
                    .reason()
                    .unwrap_or_else(|| "copy cancelled".to_string()),
            ));
        }

        bytes_transferred += source
            .copy_file(
                &source_path,
                target,
                &target_path,
                transfer_id.clone(),
                cancel.clone(),
                None,
            )
            .await?;
        progress.maybe_emit(bytes_transferred);
    }

    progress.emit_final(bytes_transferred);
    Ok(bytes_transferred)
}
