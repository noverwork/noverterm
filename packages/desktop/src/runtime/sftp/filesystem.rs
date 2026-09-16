use russh_sftp::client::fs::Metadata;
use russh_sftp::client::SftpSession as RusshSftpSession;
use russh_sftp::protocol::FileType as RusshFileType;

use super::types::classify_sftp_error;
use super::{FileEntry, FileType, SftpError};

pub async fn list_sftp_dir(
    session: &RusshSftpSession,
    path: &str,
) -> Result<Vec<FileEntry>, SftpError> {
    let resolved_path = resolve_remote_path(session, path).await?;
    let mut entries = session
        .read_dir(&resolved_path)
        .await
        .map_err(|error| classify_sftp_error("read", &resolved_path, error))?
        .filter(|entry| {
            let name = entry.file_name();
            name != "." && name != ".."
        })
        .map(|entry| file_entry_from_metadata(entry.file_name(), &entry.metadata()))
        .collect::<Vec<_>>();

    sort_file_entries(&mut entries);
    Ok(entries)
}

/// Resolve a path to an absolute path on the remote server.
///
/// If the path starts with `~`, it is resolved to the user's home directory
/// using SFTP's `realpath` command. Otherwise, the path is passed through
/// unchanged.
async fn resolve_remote_path(session: &RusshSftpSession, path: &str) -> Result<String, SftpError> {
    if path.starts_with('~') {
        session
            .canonicalize(path)
            .await
            .map_err(|error| classify_sftp_error("realpath", path, error))
    } else {
        Ok(path.to_string())
    }
}

pub async fn stat_sftp(session: &RusshSftpSession, path: &str) -> Result<FileEntry, SftpError> {
    let resolved_path = resolve_remote_path(session, path).await?;
    let metadata = session
        .metadata(&resolved_path)
        .await
        .map_err(|error| classify_sftp_error("read", &resolved_path, error))?;

    Ok(file_entry_from_metadata(
        file_name_from_path(&resolved_path),
        &metadata,
    ))
}

pub async fn mkdir_sftp(session: &RusshSftpSession, path: &str) -> Result<(), SftpError> {
    let resolved_path = resolve_remote_path(session, path).await?;
    match session.create_dir(&resolved_path).await {
        Ok(()) => Ok(()),
        // OpenSSH reports EEXIST as the generic SSH_FX_FAILURE ("Failure"), so
        // the message never mentions "exists": confirm with a stat instead.
        Err(error) => {
            if session.metadata(&resolved_path).await.is_ok() {
                Err(SftpError::AlreadyExists)
            } else {
                Err(classify_sftp_error("write", path, error))
            }
        }
    }
}

fn file_entry_from_metadata(name: String, metadata: &Metadata) -> FileEntry {
    FileEntry {
        name,
        size: metadata.len(),
        modified: metadata.mtime.map(u64::from),
        file_type: map_file_type(metadata.file_type()),
    }
}

fn map_file_type(file_type: RusshFileType) -> FileType {
    match file_type {
        RusshFileType::File => FileType::File,
        RusshFileType::Dir => FileType::Dir,
        RusshFileType::Symlink => FileType::Symlink,
        RusshFileType::Other => FileType::Other,
    }
}

pub(super) fn file_name_from_path(path: &str) -> String {
    path.rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(path)
        .to_string()
}

pub(super) fn sort_file_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|left, right| {
        file_type_sort_rank(left.file_type)
            .cmp(&file_type_sort_rank(right.file_type))
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn file_type_sort_rank(file_type: FileType) -> u8 {
    match file_type {
        FileType::Dir => 0,
        FileType::File | FileType::Symlink | FileType::Other => 1,
    }
}

pub async fn remove_sftp(session: &RusshSftpSession, path: &str) -> Result<(), SftpError> {
    let resolved_path = resolve_remote_path(session, path).await?;
    let metadata = session
        .metadata(&resolved_path)
        .await
        .map_err(|error| classify_sftp_error("read", &resolved_path, error))?;

    match metadata.file_type() {
        RusshFileType::Dir => {
            let mut entries = session
                .read_dir(&resolved_path)
                .await
                .map_err(|error| classify_sftp_error("read", &resolved_path, error))?;

            if entries.next().is_some() {
                return Err(SftpError::DirectoryNotEmpty);
            }

            session
                .remove_dir(&resolved_path)
                .await
                .map_err(|error| classify_sftp_error("write", &resolved_path, error))
        }
        _ => session.remove_file(&resolved_path).await.map_err(|error| {
            let message = error.to_string();
            if message.to_lowercase().contains("is a directory") {
                SftpError::IsADirectory
            } else {
                classify_sftp_error("write", &resolved_path, message)
            }
        }),
    }
}

pub async fn rename_sftp(
    session: &RusshSftpSession,
    old: &str,
    new: &str,
) -> Result<(), SftpError> {
    let resolved_old = resolve_remote_path(session, old).await?;
    let resolved_new = resolve_remote_path(session, new).await?;

    if session.metadata(&resolved_new).await.is_ok() {
        return Err(SftpError::AlreadyExists);
    }

    session
        .rename(&resolved_old, &resolved_new)
        .await
        .map_err(|error| {
            let message = error.to_string();
            if message.to_lowercase().contains("exists") {
                SftpError::AlreadyExists
            } else {
                classify_sftp_error("write", &resolved_old, message)
            }
        })
}
