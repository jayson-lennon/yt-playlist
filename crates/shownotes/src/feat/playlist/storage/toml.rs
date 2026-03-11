use std::{collections::HashMap, path::PathBuf, time::Duration};

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use super::super::{FileMetadata, IoError, PlaylistData, PlaylistStorage};

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
        let backend = TomlStorage::new(PathBuf::from("/nonexistent/path.toml"));
        let result = backend.load();
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.playlist.is_empty());
        assert!(data.files.is_empty());
    }

    #[test]
    fn toml_backend_load_parses_valid_toml() {
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
        let result = backend.load();
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.playlist.len(), 2);
        assert_eq!(data.files.len(), 1);
    }

    #[test]
    fn toml_backend_load_handles_empty_playlist() {
        let content = r#"
version = 1
playlist = []
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());
        let result = backend.load();
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.playlist.is_empty());
    }

    #[test]
    fn toml_backend_load_filters_negative_duration() {
        let content = r#"
version = 1
playlist = []

[[files]]
path = "video.mp4"
duration = -10.0
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());
        let result = backend.load().unwrap();
        let path = PathBuf::from("video.mp4");
        let key = path.canonicalize().unwrap_or(path);
        assert!(result.files.get(&key).unwrap().duration.is_none());
    }

    #[test]
    fn toml_backend_load_filters_zero_duration() {
        let content = r#"
version = 1
playlist = []

[[files]]
path = "video.mp4"
duration = 0.0
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());
        let result = backend.load().unwrap();
        let path = PathBuf::from("video.mp4");
        let key = path.canonicalize().unwrap_or(path);
        assert!(result.files.get(&key).unwrap().duration.is_none());
    }

    #[test]
    fn toml_backend_load_keeps_positive_duration() {
        let content = r#"
version = 1
playlist = []

[[files]]
path = "video.mp4"
duration = 120.5
"#;
        let file = create_temp_file(content);
        let backend = TomlStorage::new(file.path().to_path_buf());
        let result = backend.load().unwrap();
        let path = PathBuf::from("video.mp4");
        let key = path.canonicalize().unwrap_or(path);
        assert_eq!(
            result.files.get(&key).unwrap().duration,
            Some(Duration::from_secs_f64(120.5))
        );
    }

    #[test]
    fn toml_backend_save_writes_valid_toml() {
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
        backend.save(&original).expect("Save failed");
        let loaded = backend.load().expect("Load failed");
        assert_eq!(loaded.playlist.len(), 1);
        assert_eq!(loaded.files.len(), 1);
    }
}
