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
}

#[derive(Debug, Clone, Default)]
pub struct PlaylistData {
    pub playlist: Vec<PathBuf>,
    pub files: HashMap<PathBuf, FileMetadata>,
}

#[allow(clippy::missing_errors_doc)]
pub trait PlaylistStorageBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn load(&self) -> Result<PlaylistData, Report<IoError>>;
    fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>>;
}

#[derive(Debug, Clone)]
pub struct PlaylistStorage {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn PlaylistStorageBackend>,
}

#[allow(clippy::missing_errors_doc)]
impl PlaylistStorage {
    pub fn new(backend: Arc<dyn PlaylistStorageBackend>) -> Self {
        Self { backend }
    }

    pub fn load(&self) -> Result<PlaylistData, Report<IoError>> {
        self.backend.load()
    }

    pub fn save(&self, data: &PlaylistData) -> Result<(), Report<IoError>> {
        self.backend.save(data)
    }
}

pub struct TomlBackend {
    path: PathBuf,
}

impl TomlBackend {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl PlaylistStorageBackend for TomlBackend {
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
            .attach("failed to parse playlist.toml")?;

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
                let path = PathBuf::from(entry.path);
                let canonical = path.canonicalize().ok().unwrap_or(path);
                let duration = entry
                    .duration
                    .filter(|&d| d.is_finite() && d > 0.0)
                    .map(Duration::from_secs_f64);
                (
                    canonical,
                    FileMetadata {
                        duration,
                        alias: entry.alias,
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
            .attach("failed to serialize playlist.toml")?;

        std::fs::write(&self.path, content)
            .change_context(IoError)
            .attach_with(|| format!("path: {}", self.path.display()))?;

        Ok(())
    }
}
