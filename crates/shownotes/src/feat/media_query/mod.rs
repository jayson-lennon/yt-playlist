use std::{path::Path, sync::Arc, time::Duration};

use derive_more::Debug;
use error_stack::Report;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct MediaError;

pub trait MediaQuery: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    /// Returns an error if the media duration cannot be determined.
    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>>;
}

/// Service for querying media file metadata.
///
/// Provides an interface for extracting metadata from media files,
/// such as duration. Delegates to a backend implementation (ffprobe)
/// for actual media analysis.
#[derive(Debug, Clone)]
pub struct MediaQueryService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MediaQuery>,
}

impl MediaQueryService {
    pub fn new(backend: Arc<dyn MediaQuery>) -> Self {
        Self { backend }
    }

    /// # Errors
    /// Returns an error if the media duration cannot be determined.
    pub fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        self.backend.get_duration(path)
    }
}

pub mod providers;

pub use providers::{CachedMedia, Ffprobe};

pub use providers::FakeMediaBackend;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;

    fn path(s: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(s)
    }

    #[test]
    fn media_query_delegates_to_backend() {
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let query = MediaQueryService::new(fake.clone());

        let result = query.get_duration(&path("test.mp4"));

        assert!(result.is_ok());
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_backend_returns_cached_duration() {
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(path("cached.mp4"), Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        let result = cached.get_duration(&path("cached.mp4"));

        assert_eq!(result.unwrap(), Duration::from_secs(60));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cached_backend_falls_back_on_cache_miss() {
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let cached = CachedMedia::new(HashMap::new(), fake.clone());

        let result = cached.get_duration(&path("uncached.mp4"));

        assert_eq!(result.unwrap(), Duration::from_secs(120));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_backend_uses_canonical_path_for_lookup() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let canonical_path = temp_file.path().canonicalize().unwrap();
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(canonical_path.clone(), Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        let result = cached.get_duration(&canonical_path);

        assert_eq!(result.unwrap(), Duration::from_secs(60));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ffprobe_backend_name() {
        let backend = Ffprobe;

        let name = backend.name();

        assert_eq!(name, "ffprobe");
    }

    #[test]
    fn cached_backend_name() {
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let cached = CachedMedia::new(HashMap::new(), fake);

        let name = cached.name();

        assert_eq!(name, "cached");
    }

    #[test]
    fn fake_backend_name() {
        let fake = FakeMediaBackend::new(Duration::from_secs(120));

        let name = fake.name();

        assert_eq!(name, "fake");
    }

    #[test]
    fn cached_backend_multiple_lookups_use_cache() {
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(path("video.mp4"), Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        let _ = cached.get_duration(&path("video.mp4"));
        let _ = cached.get_duration(&path("video.mp4"));
        let _ = cached.get_duration(&path("video.mp4"));

        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cached_backend_different_paths_use_fallback() {
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(path("cached.mp4"), Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        let _ = cached.get_duration(&path("cached.mp4"));
        let _ = cached.get_duration(&path("other1.mp4"));
        let _ = cached.get_duration(&path("other2.mp4"));

        assert_eq!(fake.call_count.load(Ordering::SeqCst), 2);
    }
}
