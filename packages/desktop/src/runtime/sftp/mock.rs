use std::collections::HashMap;

use super::filesystem::{file_name_from_path, sort_file_entries};
use super::{FileEntry, FileType, SftpError, SftpSession, SftpSessionInner, TransferProgress};

mod transfers;
pub(super) use transfers::{download_mock, upload_mock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MockFileType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MockEntry {
    file_type: MockFileType,
    size: u64,
    modified: Option<u64>,
}

impl MockEntry {
    pub(super) fn file(size: u64, modified: Option<u64>) -> Self {
        Self {
            file_type: MockFileType::File,
            size,
            modified,
        }
    }

    pub(super) fn directory(modified: Option<u64>) -> Self {
        Self {
            file_type: MockFileType::Directory,
            size: 0,
            modified,
        }
    }

    pub(super) fn symlink(size: u64, modified: Option<u64>) -> Self {
        Self {
            file_type: MockFileType::Symlink,
            size,
            modified,
        }
    }
}

impl SftpSession {
    pub(super) fn mock(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::new()),
                upload_result: Ok(0),
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub(super) fn mock_with_entries(
        id: impl Into<String>,
        entries: HashMap<String, MockEntry>,
    ) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(entries),
                upload_result: Ok(0),
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub(super) fn mock_with_upload_result(
        id: impl Into<String>,
        upload_result: Result<u64, String>,
    ) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::new()),
                upload_result,
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub(super) fn mock_with_download_file(
        id: impl Into<String>,
        remote_path: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        let remote_path = remote_path.into();
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::from([(
                    remote_path.clone(),
                    MockEntry::file(data.len() as u64, None),
                )])),
                upload_result: Ok(0),
                download_result: Ok(data.len() as u64),
                remote_files: std::sync::Mutex::new(HashMap::from([(remote_path, data)])),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub(super) fn mock_with_download_error(
        id: impl Into<String>,
        remote_path: impl Into<String>,
        data: Vec<u8>,
        error: impl Into<String>,
    ) -> Self {
        let remote_path = remote_path.into();
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: None,
                entries: std::sync::Mutex::new(HashMap::from([(
                    remote_path.clone(),
                    MockEntry::file(data.len() as u64, None),
                )])),
                upload_result: Ok(0),
                download_result: Err(error.into()),
                remote_files: std::sync::Mutex::new(HashMap::from([(remote_path, data)])),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub(super) fn mock_with_close_error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner: SftpSessionInner::Mock {
                close_error: Some(error.into()),
                entries: std::sync::Mutex::new(HashMap::new()),
                upload_result: Ok(0),
                download_result: Ok(0),
                remote_files: std::sync::Mutex::new(HashMap::new()),
                progress_callbacks: std::sync::Mutex::new(Vec::new()),
            },
        }
    }

    pub(super) fn mock_contains_path(&self, path: &str) -> bool {
        match &self.inner {
            SftpSessionInner::Active(_) => false,
            SftpSessionInner::Mock { entries, .. } => entries
                .lock()
                .map(|entries| entries.contains_key(path))
                .unwrap_or(false),
        }
    }

    pub(super) fn mock_progress_events(&self) -> Vec<TransferProgress> {
        match &self.inner {
            SftpSessionInner::Active(_) => Vec::new(),
            SftpSessionInner::Mock {
                progress_callbacks, ..
            } => progress_callbacks
                .lock()
                .map(|events| events.clone())
                .unwrap_or_default(),
        }
    }
}

pub(super) fn list_mock_dir(
    entries: &HashMap<String, MockEntry>,
    path: &str,
) -> Result<Vec<FileEntry>, SftpError> {
    let directory = entries
        .get(path)
        .ok_or_else(|| SftpError::NotFound(path.to_string()))?;

    if directory.file_type != MockFileType::Directory {
        return Err(SftpError::IsADirectory);
    }

    let mut children = entries
        .iter()
        .filter_map(|(entry_path, entry)| {
            mock_child_name(path, entry_path).map(|name| (name, entry))
        })
        .filter(|(name, _)| name != "." && name != "..")
        .map(|(name, entry)| file_entry_from_mock(name, *entry))
        .collect::<Vec<_>>();

    sort_file_entries(&mut children);
    Ok(children)
}

pub(super) fn stat_mock(
    entries: &HashMap<String, MockEntry>,
    path: &str,
) -> Result<FileEntry, SftpError> {
    entries
        .get(path)
        .copied()
        .map(|entry| file_entry_from_mock(file_name_from_path(path), entry))
        .ok_or_else(|| SftpError::NotFound(path.to_string()))
}

pub(super) fn mkdir_mock(
    entries: &mut HashMap<String, MockEntry>,
    path: &str,
) -> Result<(), SftpError> {
    if entries.contains_key(path) {
        return Err(SftpError::AlreadyExists);
    }

    entries.insert(path.to_string(), MockEntry::directory(None));
    Ok(())
}

fn mock_child_name(parent: &str, path: &str) -> Option<String> {
    if parent == path {
        return None;
    }

    let prefix = if parent == "/" {
        "/".to_string()
    } else {
        format!("{}/", parent.trim_end_matches('/'))
    };

    let child = path.strip_prefix(&prefix)?;
    if child.is_empty() || child.contains('/') {
        return None;
    }

    Some(child.to_string())
}

fn file_entry_from_mock(name: String, entry: MockEntry) -> FileEntry {
    FileEntry {
        name,
        size: entry.size,
        modified: entry.modified,
        file_type: mock_file_type(entry.file_type),
    }
}

fn mock_file_type(file_type: MockFileType) -> FileType {
    match file_type {
        MockFileType::File => FileType::File,
        MockFileType::Directory => FileType::Dir,
        MockFileType::Symlink => FileType::Symlink,
    }
}

pub(super) fn remove_mock(
    entries: &mut HashMap<String, MockEntry>,
    path: &str,
) -> Result<(), SftpError> {
    let entry = entries
        .get(path)
        .copied()
        .ok_or_else(|| SftpError::NotFound(path.to_string()))?;

    match entry.file_type {
        MockFileType::File | MockFileType::Symlink => {
            entries.remove(path);
            Ok(())
        }
        MockFileType::Directory => {
            if mock_dir_has_entries(entries, path) {
                return Err(SftpError::DirectoryNotEmpty);
            }

            entries.remove(path);
            Ok(())
        }
    }
}

pub(super) fn rename_mock(
    entries: &mut HashMap<String, MockEntry>,
    old: &str,
    new: &str,
) -> Result<(), SftpError> {
    if entries.contains_key(new) {
        return Err(SftpError::AlreadyExists);
    }

    if !entries.contains_key(old) {
        return Err(SftpError::NotFound(old.to_string()));
    }

    let old_prefix = format!("{}/", old.trim_end_matches('/'));
    let moves = entries
        .keys()
        .filter(|path| *path == old || path.starts_with(&old_prefix))
        .cloned()
        .collect::<Vec<_>>();

    for source in moves {
        if let Some(entry) = entries.remove(&source) {
            let target = if source == old {
                new.to_string()
            } else {
                format!("{}{}", new.trim_end_matches('/'), &source[old.len()..])
            };
            entries.insert(target, entry);
        }
    }

    Ok(())
}

fn mock_dir_has_entries(entries: &HashMap<String, MockEntry>, path: &str) -> bool {
    let prefix = format!("{}/", path.trim_end_matches('/'));
    entries
        .keys()
        .any(|entry_path| entry_path.starts_with(&prefix))
}
