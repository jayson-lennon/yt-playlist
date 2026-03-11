use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use error_stack::Report;

use super::super::{MediaError, MediaQuery};

pub struct CachedMedia {
    cache: HashMap<PathBuf, Duration>,
    fallback: Arc<dyn MediaQuery>,
}

impl CachedMedia {
    pub fn new(cache: HashMap<PathBuf, Duration>, fallback: Arc<dyn MediaQuery>) -> Self {
        Self { cache, fallback }
    }
}

impl MediaQuery for CachedMedia {
    fn name(&self) -> &'static str {
        "cached"
    }

    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        let lookup_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(duration) = self.cache.get(&lookup_path) {
            return Ok(*duration);
        }
        self.fallback.get_duration(path)
    }
}
