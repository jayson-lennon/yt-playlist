use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct IoError;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileEntry {
    path: String,
    duration: Option<f64>,
    alias: Option<String>,
    #[serde(default)]
    is_virtual: bool,
    #[serde(default)]
    deleted: bool,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlaylistToml {
    version: u32,
    playlist: Vec<String>,
    #[serde(default)]
    files: Vec<FileEntry>,
}

impl Default for PlaylistToml {
    fn default() -> Self {
        Self {
            version: 1,
            playlist: Vec::new(),
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub duration: Option<Duration>,
    pub alias: Option<String>,
    pub is_virtual: bool,
    pub deleted: bool,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PlaylistData {
    pub playlist: Vec<PathBuf>,
    pub files: HashMap<PathBuf, FileMetadata>,
}

pub trait PlaylistStorage: Send + Sync {
    /// Returns the name identifier for this storage backend implementation.
    fn name(&self) -> &'static str;

    /// Loads the playlist data from persistent storage, including the file list and metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be loaded from storage.
    fn load(&self) -> Result<PlaylistData, Report<IoError>>;

    /// Persists the playlist data to storage, including the file list and all metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved to storage.
    fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>>;
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
    /// Returns an error if the playlist data cannot be loaded from the backend.
    pub fn load(&self) -> Result<PlaylistData, Report<IoError>> {
        self.backend.load()
    }

    /// # Errors
    ///
    /// Returns an error if the playlist data cannot be saved to the backend.
    pub fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.backend.save(data)
    }
}

pub struct TomlStorage {
    path: PathBuf,
}

impl TomlStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl PlaylistStorage for TomlStorage {
    fn name(&self) -> &'static str {
        "toml"
    }

    fn load(&self) -> Result<PlaylistData, Report<IoError>> {
        if !self.path.exists() {
            return Ok(PlaylistData::default());
        }

        let content = std::fs::read_to_string(&self.path)
            .change_context(IoError)
            .attach_with(|| format!("path: {}", self.path.display()))?;

        let toml: PlaylistToml = toml::from_str(&content)
            .change_context(IoError)
            .attach("failed to parse shownotes.toml")?;

        let playlist: Vec<PathBuf> = toml
            .playlist
            .into_iter()
            .filter_map(|p| {
                let path = PathBuf::from(p);
                path.canonicalize().ok().or(Some(path))
            })
            .collect();

        let files: HashMap<PathBuf, FileMetadata> = toml
            .files
            .into_iter()
            .map(|entry| {
                let path = PathBuf::from(&entry.path);
                let canonical = if entry.is_virtual {
                    path
                } else {
                    path.canonicalize().ok().unwrap_or(path)
                };
                let duration = entry
                    .duration
                    .filter(|&d| d.is_finite() && d > 0.0)
                    .map(Duration::from_secs_f64);
                (
                    canonical,
                    FileMetadata {
                        duration,
                        alias: entry.alias,
                        is_virtual: entry.is_virtual,
                        deleted: entry.deleted,
                        mime_type: entry.mime_type,
                    },
                )
            })
            .collect();

        Ok(PlaylistData { playlist, files })
    }

    fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        let files: Vec<FileEntry> = data
            .files
            .iter()
            .map(|(k, v)| FileEntry {
                path: k.to_string_lossy().into_owned(),
                duration: v.duration.map(|d| d.as_secs_f64()),
                alias: v.alias.clone(),
                is_virtual: v.is_virtual,
                deleted: v.deleted,
                mime_type: v.mime_type.clone(),
            })
            .collect();

        let toml = PlaylistToml {
            version: 1,
            playlist: data
                .playlist
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            files,
        };

        let content = toml::to_string_pretty(&toml)
            .change_context(IoError)
            .attach("failed to serialize shownotes.toml")?;

        std::fs::write(&self.path, content)
            .change_context(IoError)
            .attach_with(|| format!("path: {}", self.path.display()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to temp file");
        file.flush().expect("Failed to flush temp file");
        file
    }

    #[test]
    fn toml_backend_load_returns_default_when_file_not_exists() {
        // Given a backend with non-existent path.
        let backend = TomlStorage::new(PathBuf::from("/nonexistent/path.toml"));

        // When loading.
        let result = backend.load();

        // Then default data is returned.
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.playlist.is_empty());
        assert!(data.files.is_empty());
    }

    #[test]
    fn toml_backend_load_parses_valid_toml() {
        // Given a valid TOML file.
        let content = r#"
version = 1
playlist = ["video1.mp4", "video2.mp4"]

[[files]]
path = "video1.mp4"
duration = 120.5
alias = "First Video"
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());

        // When loading.
        let result = backend.load();

        // Then data is parsed correctly.
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.playlist.len(), 2);
        assert_eq!(data.files.len(), 1);
    }

    #[test]
    fn toml_backend_load_handles_empty_playlist() {
        // Given an empty TOML file.
        let content = r#"
version = 1
playlist = []
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());

        // When loading.
        let result = backend.load();

        // Then empty data is returned.
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.playlist.is_empty());
    }

    #[test]
    fn toml_backend_load_filters_negative_duration() {
        // Given a TOML file with negative duration.
        let content = r#"
version = 1
playlist = []

[[files]]
path = "video.mp4"
duration = -10.0
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());

        // When loading.
        let result = backend.load().unwrap();

        // Then duration is filtered out.
        let path = PathBuf::from("video.mp4");
        let key = path.canonicalize().unwrap_or(path);
        assert!(result.files.get(&key).unwrap().duration.is_none());
    }

    #[test]
    fn toml_backend_load_filters_zero_duration() {
        // Given a TOML file with zero duration.
        let content = r#"
version = 1
playlist = []

[[files]]
path = "video.mp4"
duration = 0.0
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());

        // When loading.
        let result = backend.load().unwrap();

        // Then duration is filtered out.
        let path = PathBuf::from("video.mp4");
        let key = path.canonicalize().unwrap_or(path);
        assert!(result.files.get(&key).unwrap().duration.is_none());
    }

    #[test]
    fn toml_backend_load_keeps_positive_duration() {
        // Given a TOML file with positive duration.
        let content = r#"
version = 1
playlist = []

[[files]]
path = "video.mp4"
duration = 120.5
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());

        // When loading.
        let result = backend.load().unwrap();

        // Then duration is kept.
        let path = PathBuf::from("video.mp4");
        let key = path.canonicalize().unwrap_or(path);
        assert_eq!(
            result.files.get(&key).unwrap().duration,
            Some(Duration::from_secs_f64(120.5))
        );
    }

    #[test]
    fn toml_backend_save_writes_valid_toml() {
        // Given a temp file and data.
        let file = NamedTempFile::new().expect("Failed to create temp file");
        let backend = TomlStorage::new(file.path().to_path_buf());
        let mut original = PlaylistData::default();
        original.playlist.push(PathBuf::from("video.mp4"));
        original.files.insert(
            PathBuf::from("video.mp4"),
            FileMetadata {
                duration: Some(Duration::from_secs(120)),
                alias: Some("My Video".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );

        // When saving and loading.
        backend.save(&original).expect("Save failed");
        let loaded = backend.load().expect("Load failed");

        // Then data is preserved.
        assert_eq!(loaded.playlist.len(), 1);
        assert_eq!(loaded.files.len(), 1);
    }

    #[test]
    fn playlist_storage_delegates_to_backend() {
        // Given a storage with fake backend.
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());

        // When loading.
        let result = storage.load();

        // Then backend is called.
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
        // Given a storage with fake backend.
        let backend = Arc::new(FakeStorageBackend::default());
        let storage = PlaylistStorageService::new(backend.clone());
        let data = PlaylistData::default();

        // When saving.
        let result = storage.save(&data);

        // Then backend is called.
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
        // Given file metadata.
        let metadata = FileMetadata {
            duration: Some(Duration::from_secs(120)),
            alias: Some("test".to_string()),
            is_virtual: false,
            deleted: false,
            mime_type: None,
        };

        // When cloning.
        let cloned = metadata.clone();

        // Then values are preserved.
        assert_eq!(cloned.duration, metadata.duration);
        assert_eq!(cloned.alias, metadata.alias);
    }

    #[test]
    fn playlist_data_default_is_empty() {
        // Given default playlist data.
        let data = PlaylistData::default();

        // Then it is empty.
        assert!(data.playlist.is_empty());
        assert!(data.files.is_empty());
    }

    struct FakeStorageBackend {
        load_called: std::sync::atomic::AtomicUsize,
        save_called: std::sync::atomic::AtomicUsize,
    }

    impl FakeStorageBackend {
        fn new() -> Self {
            Self {
                load_called: std::sync::atomic::AtomicUsize::new(0),
                save_called: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    impl Default for FakeStorageBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PlaylistStorage for FakeStorageBackend {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn load(&self) -> Result<PlaylistData, Report<IoError>> {
            self.load_called
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(PlaylistData::default())
        }

        fn save(&self, _data: &PlaylistData) -> Result<(), Report<IoError>> {
            self.save_called
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }
}
