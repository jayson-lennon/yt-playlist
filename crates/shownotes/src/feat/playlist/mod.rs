use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use derive_more::Debug;
use error_stack::Report;
use jiff::Timestamp;
use marked_path::CanonicalPath;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct IoError;

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub duration: Option<std::time::Duration>,
    pub is_virtual: bool,
    pub deleted: bool,
    pub mime_type: Option<String>,
    pub time_added: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct PlaylistData {
    pub working_directory: CanonicalPath,
    pub playlist: Vec<PathBuf>,
    pub files: HashMap<PathBuf, FileMetadata>,
}

#[async_trait]
pub trait PlaylistStorage: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be loaded.
    async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>>;

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved.
    async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>>;
}

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
    pub async fn load(&self, working_directory: &CanonicalPath) -> Result<PlaylistData, Report<IoError>> {
        self.backend.load(working_directory).await
    }

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved.
    pub async fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.backend.save(data).await
    }
}

pub mod storage;
pub use storage::FakeStorageBackend;
pub use storage::SqliteStorage;

#[cfg(test)]
mod tests {
    use super::*;
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
        };
        let cloned = metadata.clone();
        assert_eq!(cloned.duration, metadata.duration);
        assert_eq!(cloned.time_added, metadata.time_added);
    }
}
