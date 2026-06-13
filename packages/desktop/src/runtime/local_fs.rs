//! Local filesystem operations bridge for the dual-panel file browser.
//!
//! This module mirrors the SFTP operations API to enable seamless switching
//! between local and remote (SFTP) file browsing.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use tokio::fs;

use super::sftp::{FileEntry, FileType};

/// Expand `~` to the user's home directory.
pub(crate) fn expand_tilde(path: &str) -> Result<PathBuf, String> {
    if path == "~" {
        home_dir().map(PathBuf::from)
    } else if let Some(rest) = path.strip_prefix("~/") {
        home_dir().map(|home| PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

fn home_dir() -> Result<String, String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "HOME environment variable not set".to_string())
}

fn file_type_from_metadata(metadata: &std::fs::Metadata) -> FileType {
    if metadata.file_type().is_dir() {
        FileType::Dir
    } else if metadata.file_type().is_symlink() {
        FileType::Symlink
    } else if metadata.file_type().is_file() {
        FileType::File
    } else {
        FileType::Other
    }
}

async fn entry_from_dir_entry(entry: &tokio::fs::DirEntry) -> std::io::Result<FileEntry> {
    let metadata = entry.metadata().await?;
    let name = entry.file_name().to_str().unwrap_or_default().to_string();

    let file_type = file_type_from_metadata(&metadata);
    let size = if matches!(
        file_type,
        FileType::Dir | FileType::Symlink | FileType::Other
    ) {
        0
    } else {
        metadata.len()
    };

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    Ok(FileEntry {
        name,
        size,
        modified,
        file_type,
    })
}

fn entry_from_path(path: &Path, name: &str) -> std::io::Result<FileEntry> {
    let metadata = path.metadata()?;
    let file_type = file_type_from_metadata(&metadata);
    let size = if matches!(
        file_type,
        FileType::Dir | FileType::Symlink | FileType::Other
    ) {
        0
    } else {
        metadata.len()
    };

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    Ok(FileEntry {
        name: name.to_string(),
        size,
        modified,
        file_type,
    })
}

/// List directory contents.
///
/// Returns entries sorted: directories first, then files, alphabetically.
pub async fn local_list_dir(path: &str) -> Result<Vec<FileEntry>, String> {
    let expanded_path = expand_tilde(path)?;
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(&expanded_path).await.map_err(|e| {
        format!(
            "Failed to open directory '{}': {}",
            expanded_path.display(),
            e
        )
    })?;

    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read directory entry: {}", e))?
    {
        let name = entry.file_name().to_string_lossy().to_string();

        if name == "." || name == ".." {
            continue;
        }

        match entry_from_dir_entry(&entry).await {
            Ok(file_entry) => entries.push(file_entry),
            Err(e) => {
                tracing::warn!(name, "Failed to get entry metadata: {}", e);
            }
        }
    }

    entries.sort_by(|a, b| match (a.file_type, b.file_type) {
        (FileType::Dir, _) => std::cmp::Ordering::Less,
        (_, FileType::Dir) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

/// Get file/directory metadata.
pub async fn local_stat(path: &str) -> Result<FileEntry, String> {
    let expanded_path = expand_tilde(path)?;
    let p = &expanded_path;

    if !p.exists() {
        return Err(format!("Path does not exist: {}", expanded_path.display()));
    }

    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| expanded_path.to_string_lossy().to_string());

    entry_from_path(p, &name)
        .map_err(|e| format!("Failed to stat '{}': {}", expanded_path.display(), e))
}

/// Create a directory.
pub async fn local_mkdir(path: &str) -> Result<(), String> {
    let expanded_path = expand_tilde(path)?;
    fs::create_dir(&expanded_path).await.map_err(|e| {
        format!(
            "Failed to create directory '{}': {}",
            expanded_path.display(),
            e
        )
    })
}

/// Remove a file or empty directory.
pub async fn local_remove(path: &str) -> Result<(), String> {
    let expanded_path = expand_tilde(path)?;
    let p = &expanded_path;

    if !p.exists() {
        return Err(format!("Path does not exist: {}", expanded_path.display()));
    }

    if p.is_dir() {
        match fs::remove_dir(p).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::DirectoryNotEmpty {
                    return Err(format!("Directory not empty: {}", expanded_path.display()));
                }
                if p.is_file() {
                    return fs::remove_file(p).await.map_err(|e| {
                        format!("Failed to remove '{}': {}", expanded_path.display(), e)
                    });
                }
                return Err(format!(
                    "Failed to remove '{}': {}",
                    expanded_path.display(),
                    e
                ));
            }
        }
    }

    fs::remove_file(p)
        .await
        .map_err(|e| format!("Failed to remove '{}': {}", expanded_path.display(), e))
}

/// Rename a file or directory.
pub async fn local_rename(old: &str, new: &str) -> Result<(), String> {
    let old_expanded = expand_tilde(old)?;
    let new_expanded = expand_tilde(new)?;
    fs::rename(&old_expanded, &new_expanded).await.map_err(|e| {
        format!(
            "Failed to rename '{}' to '{}': {}",
            old_expanded.display(),
            new_expanded.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_list_dir() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let path = temp.path().to_str().unwrap();

        tokio::fs::write(temp.path().join("file1.txt"), "content1")
            .await
            .expect("Failed to create file1");
        tokio::fs::write(temp.path().join("file2.txt"), "content2")
            .await
            .expect("Failed to create file2");
        tokio::fs::create_dir(temp.path().join("subdir"))
            .await
            .expect("Failed to create subdir");

        let entries = local_list_dir(path).await.expect("list_dir should succeed");

        assert_eq!(entries.len(), 3);

        let subdir = entries
            .iter()
            .find(|e| e.name == "subdir")
            .expect("subdir should exist");
        assert_eq!(subdir.file_type, FileType::Dir);

        let file1 = entries
            .iter()
            .find(|e| e.name == "file1.txt")
            .expect("file1.txt should exist");
        assert_eq!(file1.file_type, FileType::File);
        assert_eq!(file1.size, 8);
    }

    #[tokio::test]
    async fn test_local_list_empty() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let path = temp.path().to_str().unwrap();

        let entries = local_list_dir(path).await.expect("list_dir should succeed");
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_local_list_tilde_expansion() {
        let result = local_list_dir("~").await;
        if let Ok(home) = std::env::var("HOME") {
            assert!(
                result.is_ok(),
                "tilde should expand to home directory: {}",
                home
            );
        }
    }

    #[tokio::test]
    async fn test_expand_tilde_variants() {
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(expand_tilde("~").unwrap(), PathBuf::from(&home));
            assert_eq!(
                expand_tilde("~/test").unwrap(),
                PathBuf::from(&home).join("test")
            );
            assert_eq!(
                expand_tilde("/absolute/path").unwrap(),
                PathBuf::from("/absolute/path")
            );
            assert_eq!(
                expand_tilde("relative/path").unwrap(),
                PathBuf::from("relative/path")
            );
        }
    }

    #[tokio::test]
    async fn test_local_stat_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp.path().join("test.txt");
        tokio::fs::write(&file_path, "hello world")
            .await
            .expect("Failed to create file");

        let entry = local_stat(file_path.to_str().unwrap())
            .await
            .expect("stat should succeed");

        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.file_type, FileType::File);
        assert_eq!(entry.size, 11);
    }

    #[tokio::test]
    async fn test_local_stat_nonexistent() {
        let result = local_stat("/nonexistent/path/to/file").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_mkdir() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let new_dir = temp.path().join("new_directory");

        local_mkdir(new_dir.to_str().unwrap())
            .await
            .expect("mkdir should succeed");

        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[tokio::test]
    async fn test_local_mkdir_existing() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let existing = temp.path().join("existing");

        tokio::fs::create_dir(&existing)
            .await
            .expect("Failed to create existing dir");

        let result = local_mkdir(existing.to_str().unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_remove_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp.path().join("to_remove.txt");

        tokio::fs::write(&file_path, "content")
            .await
            .expect("Failed to create file");
        assert!(file_path.exists());

        local_remove(file_path.to_str().unwrap())
            .await
            .expect("remove should succeed");

        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_local_remove_empty_dir() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let dir_path = temp.path().join("empty_dir");

        tokio::fs::create_dir(&dir_path)
            .await
            .expect("Failed to create dir");
        assert!(dir_path.exists());

        local_remove(dir_path.to_str().unwrap())
            .await
            .expect("remove should succeed");

        assert!(!dir_path.exists());
    }

    #[tokio::test]
    async fn test_local_remove_directory_not_empty() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let dir_path = temp.path().join("not_empty");

        tokio::fs::create_dir(&dir_path)
            .await
            .expect("Failed to create dir");
        tokio::fs::write(dir_path.join("file.txt"), "content")
            .await
            .expect("Failed to create file in dir");

        let result = local_remove(dir_path.to_str().unwrap()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Directory not empty"));
    }

    #[tokio::test]
    async fn test_local_rename() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let old_path = temp.path().join("old_name.txt");
        let new_path = temp.path().join("new_name.txt");

        tokio::fs::write(&old_path, "content")
            .await
            .expect("Failed to create file");
        assert!(old_path.exists());
        assert!(!new_path.exists());

        local_rename(old_path.to_str().unwrap(), new_path.to_str().unwrap())
            .await
            .expect("rename should succeed");

        assert!(!old_path.exists());
        assert!(new_path.exists());
    }
}
