use std::{collections::HashMap, path::PathBuf, time::Duration};

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use wherror::Error;

#[derive(Debug, Error)]
#[error("cache error")]
pub struct CacheError;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    files: HashMap<String, f64>,
}

impl Default for CacheFile {
    fn default() -> Self {
        Self {
            version: 1,
            files: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DurationCache {
    path: PathBuf,
    data: HashMap<PathBuf, Duration>,
}

impl DurationCache {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            data: HashMap::new(),
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn load(path: PathBuf) -> Result<Self, Report<CacheError>> {
        let mut cache = Self::new(path.clone());
        if !cache.path.exists() {
            return Ok(cache);
        }
        let content = std::fs::read_to_string(&path)
            .change_context(CacheError)
            .attach("failed to read cache file")?;
        let file: CacheFile = toml::from_str(&content)
            .change_context(CacheError)
            .attach("failed to parse cache file")?;
        cache.data = file
            .files
            .into_iter()
            .filter_map(|(k, v)| {
                let path = PathBuf::from(k);
                if v.is_finite() && v > 0.0 {
                    Some((path, Duration::from_secs_f64(v)))
                } else {
                    None
                }
            })
            .collect();
        Ok(cache)
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn save(&self) -> Result<(), Report<CacheError>> {
        let file = CacheFile {
            version: 1,
            files: self
                .data
                .iter()
                .map(|(k, v)| (k.to_string_lossy().into_owned(), v.as_secs_f64()))
                .collect(),
        };
        let content = toml::to_string_pretty(&file)
            .change_context(CacheError)
            .attach("failed to serialize cache")?;
        std::fs::write(&self.path, content)
            .change_context(CacheError)
            .attach("failed to write cache file")?;
        Ok(())
    }

    pub fn get(&self, path: &PathBuf) -> Option<Duration> {
        self.data.get(path).copied()
    }

    pub fn insert(&mut self, path: PathBuf, duration: Duration) {
        self.data.insert(path, duration);
    }

    pub fn contains(&self, path: &PathBuf) -> bool {
        self.data.contains_key(path)
    }
}
