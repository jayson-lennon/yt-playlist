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

mod providers;

pub use providers::{CachedMedia, Ffprobe};

pub use providers::FakeMediaBackend;

#[cfg(test)]
mod tests {
    use super::*;
    use marked_path::CanonicalPath;
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;

    fn path(s: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(s)
    }

    #[test]
    fn media_query_delegates_to_backend() {
        // Given a service with a fake backend.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let query = MediaQueryService::new(fake.clone());

        // When calling the service method.
        let result = query.get_duration(&path("test.mp4"));

        // Then the backend was called and result is successful.
        assert!(result.is_ok());
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_backend_returns_cached_duration() {
        // Given a cached backend with a pre-populated cache entry.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(canonical.clone(), Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        // When getting duration for the cached file.
        let result = cached.get_duration(temp_file.path());

        // Then the cached duration is returned without calling the backend.
        assert_eq!(result.unwrap(), Duration::from_secs(60));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cached_backend_falls_back_on_cache_miss() {
        // Given a cached backend with an empty cache.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let cached = CachedMedia::new(HashMap::new(), fake.clone());

        // When getting duration for an uncached file.
        let result = cached.get_duration(&path("uncached.mp4"));

        // Then the backend is called and returns its duration.
        assert_eq!(result.unwrap(), Duration::from_secs(120));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_backend_uses_canonical_path_for_lookup() {
        // Given a cached backend with a cache entry keyed by canonical path.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(canonical.clone(), Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        // When getting duration using the non-canonical path.
        let result = cached.get_duration(temp_file.path());

        // Then the cache lookup succeeds using the canonical path.
        assert_eq!(result.unwrap(), Duration::from_secs(60));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ffprobe_backend_name() {
        // Given an ffprobe backend.
        let backend = Ffprobe;

        // When getting the backend name.
        let name = backend.name();

        // Then the name is "ffprobe".
        assert_eq!(name, "ffprobe");
    }

    #[test]
    fn cached_backend_name() {
        // Given a cached backend wrapping a fake backend.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let cached = CachedMedia::new(HashMap::new(), fake);

        // When getting the backend name.
        let name = cached.name();

        // Then the name is "cached".
        assert_eq!(name, "cached");
    }

    #[test]
    fn fake_backend_name() {
        // Given a fake backend.
        let fake = FakeMediaBackend::new(Duration::from_secs(120));

        // When getting the backend name.
        let name = fake.name();

        // Then the name is "fake".
        assert_eq!(name, "fake");
    }

    #[test]
    fn cached_backend_multiple_lookups_use_cache() {
        // Given a cached backend with a pre-populated cache entry.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(canonical, Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        // When getting duration multiple times for the same file.
        cached.get_duration(temp_file.path()).unwrap();
        cached.get_duration(temp_file.path()).unwrap();
        cached.get_duration(temp_file.path()).unwrap();

        // Then the backend is never called.
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cached_backend_different_paths_use_fallback() {
        // Given a cached backend with a cache entry for one file only.
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let canonical = CanonicalPath::from_path(temp_file.path()).unwrap();
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(canonical, Duration::from_secs(60));
        let cached = CachedMedia::new(cache, fake.clone());

        // When getting duration for the cached file and two uncached files.
        cached.get_duration(temp_file.path()).unwrap();
        cached.get_duration(&path("other1.mp4")).unwrap();
        cached.get_duration(&path("other2.mp4")).unwrap();

        // Then the backend is called only for the uncached files.
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 2);
    }
}
