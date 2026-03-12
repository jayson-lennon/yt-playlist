use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use error_stack::Report;
use marked_path::CanonicalPath;

use super::super::{MediaError, MediaQuery};

pub struct CachedMedia {
    cache: HashMap<CanonicalPath, Duration>,
    fallback: Arc<dyn MediaQuery>,
}

impl CachedMedia {
    pub fn new(cache: HashMap<CanonicalPath, Duration>, fallback: Arc<dyn MediaQuery>) -> Self {
        Self { cache, fallback }
    }
}

impl MediaQuery for CachedMedia {
    fn name(&self) -> &'static str {
        "cached"
    }

    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        if let Ok(canonical) = CanonicalPath::from_path(path) {
            if let Some(duration) = self.cache.get(&canonical) {
                return Ok(*duration);
            }
        }
        self.fallback.get_duration(path)
    }
}
