use std::{path::PathBuf, sync::Arc};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct IoError;

#[allow(clippy::missing_errors_doc)]
pub trait PlaylistStorageBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn load(&self) -> Result<Vec<PathBuf>, Report<IoError>>;
    fn save(&self, items: &[PathBuf]) -> Result<(), Report<IoError>>;
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

    pub fn load(&self) -> Result<Vec<PathBuf>, Report<IoError>> {
        self.backend.load()
    }

    pub fn save(&self, items: &[PathBuf]) -> Result<(), Report<IoError>> {
        self.backend.save(items)
    }
}

pub struct FileBackend {
    path: PathBuf,
}

impl FileBackend {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl PlaylistStorageBackend for FileBackend {
    fn name(&self) -> &'static str {
        "file"
    }

    fn load(&self) -> Result<Vec<PathBuf>, Report<IoError>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.path)
            .change_context(IoError)
            .attach_with(|| format!("path: {}", self.path.display()))?;
        let items = content
            .lines()
            .filter(|line: &&str| !line.is_empty())
            .map(PathBuf::from)
            .collect();
        Ok(items)
    }

    fn save(&self, items: &[PathBuf]) -> Result<(), Report<IoError>> {
        let content = items
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&self.path, content)
            .change_context(IoError)
            .attach_with(|| format!("path: {}", self.path.display()))?;
        Ok(())
    }
}
