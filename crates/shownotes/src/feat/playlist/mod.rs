//! Playlist storage and management.
//!
//! This module handles persisting and loading playlist data, including file metadata
//! and display aliases.
//!
//! # Notes vs Aliases
//!
//! Notes and aliases are separate concepts:
//!
//! - **Notes**: Searchable metadata attached to files for discovery purposes. Notes are
//!   never displayed in the TUI but can be searched to find files. They are stored in
//!   the `notes` table.
//!
//! - **Aliases**: Display names shown in the TUI as if they were the filename. When a
//!   user renames a file in the TUI, the alias is stored in the `aliases` table. For
//!   convenience, aliases are also saved to the notes table so users can search by
//!   alias name.
//!
//! # Alias Resolution Priority
//!
//! When displaying a file, aliases are resolved with the following priority:
//!
//! 1. **Workspace-specific alias**: If an alias exists for the current workspace, use it
//! 2. **Most recent alias**: If no workspace-specific alias exists, use the most recently
//!    updated alias from any workspace
//! 3. **Filename**: If no alias exists at all, display the actual filename

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use jiff::Timestamp;
use marked_path::CanonicalPath;
use wherror::Error;

use crate::common::domain::ItemPath;

#[derive(Debug, Error)]
#[error(debug)]
pub struct IoError;

/// Metadata associated with a file in the playlist or library.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Duration of the media file, if available.
    pub duration: Option<std::time::Duration>,
    /// Whether this is a virtual file (URL) that doesn't exist on disk.
    pub is_virtual: bool,
    /// Whether the file has been deleted from disk.
    pub deleted: bool,
    /// MIME type of the file.
    pub mime_type: Option<String>,
    /// Timestamp when the file was added to the playlist.
    pub time_added: Option<Timestamp>,
    /// Display alias for the file, shown in the TUI instead of the filename.
    ///
    /// See [module documentation](self) for alias resolution priority.
    pub alias: Option<String>,
}

/// Complete playlist data for a workspace.
#[derive(Debug, Clone)]
pub struct PlaylistData {
    /// The working directory (workspace) for this playlist.
    pub working_directory: CanonicalPath,
    /// Ordered list of file paths in the playlist.
    pub playlist: Vec<ItemPath>,
    /// Metadata for all files (playlist items and library files).
    pub files: HashMap<ItemPath, FileMetadata>,
}

/// Storage backend for playlist data.
///
/// Implementations persist playlist state including file order, metadata, and aliases.
#[async_trait]
pub trait PlaylistStorage: Send + Sync {
    /// Returns the name of this storage backend for debugging.
    fn name(&self) -> &'static str;

    /// Loads playlist data for the given workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be loaded.
    async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>>;

    /// Saves playlist data for a workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved.
    async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>>;

    /// Saves an alias for a file in a specific workspace.
    ///
    /// The alias will be displayed in the TUI instead of the filename.
    /// If an alias already exists for this file/workspace combination, it will be updated.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias cannot be saved.
    async fn upsert_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
        alias: &str,
    ) -> Result<(), Report<IoError>>;

    /// Deletes the alias for a file in a specific workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias cannot be deleted.
    async fn delete_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<(), Report<IoError>>;

    /// Resolves the display alias for a file in a specific workspace.
    ///
    /// # Alias Resolution Priority
    ///
    /// 1. Workspace-specific alias (if exists for current workspace)
    /// 2. Most recently updated alias from any workspace (fallback)
    /// 3. `None` (caller should display filename instead)
    ///
    /// # Errors
    ///
    /// Returns an error if the alias cannot be resolved.
    async fn resolve_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<Option<String>, Report<IoError>>;
}

/// Service wrapper for playlist storage operations.
#[derive(Debug, Clone)]
pub struct PlaylistStorageService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn PlaylistStorage>,
}

impl PlaylistStorageService {
    /// Creates a new playlist storage service with the given backend.
    pub fn new(backend: Arc<dyn PlaylistStorage>) -> Self {
        Self { backend }
    }

    /// Loads playlist data for the given workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be loaded.
    pub async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>> {
        self.backend.load(working_directory).await
    }

    /// Saves playlist data for a workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved.
    pub async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.backend.save(data).await
    }

    /// Saves an alias for a file in a specific workspace.
    ///
    /// See [`PlaylistStorage::upsert_alias`] for details.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias cannot be saved.
    pub async fn upsert_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
        alias: &str,
    ) -> Result<(), Report<IoError>> {
        self.backend.upsert_alias(file_path, workspace, alias).await
    }

    /// Deletes the alias for a file in a specific workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias cannot be deleted.
    pub async fn delete_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<(), Report<IoError>> {
        self.backend.delete_alias(file_path, workspace).await
    }

    /// Resolves the display alias for a file in a specific workspace.
    ///
    /// See [`PlaylistStorage::resolve_alias`] for alias resolution priority.
    ///
    /// # Errors
    ///
    /// Returns an error if the alias cannot be resolved.
    pub async fn resolve_alias(
        &self,
        file_path: &CanonicalPath,
        workspace: &CanonicalPath,
    ) -> Result<Option<String>, Report<IoError>> {
        self.backend.resolve_alias(file_path, workspace).await
    }
}

mod storage;
pub use storage::FakeStorageBackend;
pub use storage::SqliteStorage;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn playlist_storage_delegates_to_backend() {
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());
        let result = storage.load(&working_dir).await;
        assert!(result.is_ok());
        assert_eq!(
            backend
                .load_called
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[tokio::test]
    async fn playlist_storage_save_delegates_to_backend() {
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());
        let data = PlaylistData {
            working_directory: working_dir,
            playlist: Vec::new(),
            files: HashMap::new(),
        };
        let result = storage.save(&data).await;
        assert!(result.is_ok());
        assert_eq!(
            backend
                .save_called
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn file_metadata_can_be_cloned() {
        let metadata = FileMetadata {
            duration: Some(Duration::from_secs(120)),
            is_virtual: false,
            deleted: false,
            mime_type: None,
            time_added: None,
            alias: Some("My Video".to_string()),
        };
        let cloned = metadata.clone();
        assert_eq!(cloned.duration, metadata.duration);
        assert_eq!(cloned.time_added, metadata.time_added);
        assert_eq!(cloned.alias, metadata.alias);
    }

    #[tokio::test]
    async fn playlist_storage_upsert_alias_delegates_to_backend() {
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());

        let file = CanonicalPath::new(PathBuf::from("/test/video.mp4"));
        let result = storage
            .upsert_alias(&file, &working_dir, "My Video")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn playlist_storage_resolve_alias_delegates_to_backend() {
        let temp = TempDir::new().unwrap();
        let working_dir = CanonicalPath::from_path(temp.path()).unwrap();
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());

        let file = CanonicalPath::new(PathBuf::from("/test/video.mp4"));
        storage
            .upsert_alias(&file, &working_dir, "My Video")
            .await
            .unwrap();

        let result = storage.resolve_alias(&file, &working_dir).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("My Video".to_string()));
    }
}
