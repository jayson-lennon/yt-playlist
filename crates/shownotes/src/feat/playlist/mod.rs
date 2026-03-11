use std::{collections::HashMap, path::PathBuf, sync::Arc};

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct IoError;

/// Metadata associated with a file in the playlist.
///
/// Stores optional information about a file including its duration,
/// display alias, whether it's a virtual item (URL), deletion status,
/// and MIME type.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub duration: Option<std::time::Duration>,
    pub alias: Option<String>,
    pub is_virtual: bool,
    pub deleted: bool,
    pub mime_type: Option<String>,
}

/// Complete playlist data including order and file metadata.
///
/// Contains the ordered list of file paths in the playlist and a map
/// of metadata for each file. This structure is serialized to and
/// deserialized from the playlist storage file.
#[derive(Debug, Clone, Default)]
pub struct PlaylistData {
    pub playlist: Vec<PathBuf>,
    pub files: HashMap<PathBuf, FileMetadata>,
}

pub trait PlaylistStorage: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be loaded.
    fn load(&self) -> Result<PlaylistData, Report<IoError>>;

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved.
    fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>>;
}

/// Service for loading and saving playlist data.
///
/// Wraps a storage backend to provide a simple interface for persisting
/// playlist state. Delegates actual storage operations to the backend
/// implementation (e.g., TOML file storage).
#[derive(Debug, Clone)]
pub struct PlaylistStorageService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn PlaylistStorage>,
}

impl PlaylistStorageService {
    pub fn new(backend: Arc<dyn PlaylistStorage>) -> Self {
        Self { backend }
    }

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be loaded.
    pub fn load(&self) -> Result<PlaylistData, Report<IoError>> {
        self.backend.load()
    }

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved.
    pub fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.backend.save(data)
    }
}

pub mod storage;
pub use storage::TomlStorage;

pub use storage::FakeStorageBackend;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn playlist_storage_delegates_to_backend() {
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());
        let result = storage.load();
        assert!(result.is_ok());
        assert_eq!(
            backend
                .load_called
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn playlist_storage_save_delegates_to_backend() {
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());
        let data = PlaylistData::default();
        let result = storage.save(&data);
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
            alias: Some("test".to_string()),
            is_virtual: false,
            deleted: false,
            mime_type: None,
        };
        let cloned = metadata.clone();
        assert_eq!(cloned.duration, metadata.duration);
        assert_eq!(cloned.alias, metadata.alias);
    }

    #[test]
    fn playlist_data_default_is_empty() {
        let data = PlaylistData::default();
        assert!(data.playlist.is_empty());
        assert!(data.files.is_empty());
    }
}
