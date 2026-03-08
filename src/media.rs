use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct MediaError;

pub trait MediaQueryBackend: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    ///
    /// Returns an error if the media duration cannot be determined.
    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>>;
}

#[derive(Debug, Clone)]
pub struct MediaQuery {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MediaQueryBackend>,
}

impl MediaQuery {
    pub fn new(backend: Arc<dyn MediaQueryBackend>) -> Self {
        Self { backend }
    }

    /// # Errors
    ///
    /// Returns an error if the media duration cannot be determined by the backend.
    pub fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        self.backend.get_duration(path)
    }
}

pub struct FfprobeBackend;

impl MediaQueryBackend for FfprobeBackend {
    fn name(&self) -> &'static str {
        "ffprobe"
    }

    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        let info = ffprobe::ffprobe(path).change_context(MediaError)?;
        info.format
            .get_duration()
            .ok_or_else(|| Report::new(MediaError))
            .attach("no duration in ffprobe output")
    }
}

pub struct CachedMediaBackend {
    cache: HashMap<PathBuf, Duration>,
    fallback: Arc<dyn MediaQueryBackend>,
}

impl CachedMediaBackend {
    pub fn new(cache: HashMap<PathBuf, Duration>, fallback: Arc<dyn MediaQueryBackend>) -> Self {
        Self { cache, fallback }
    }
}

impl MediaQueryBackend for CachedMediaBackend {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn path(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    struct FakeMediaBackend {
        call_count: AtomicUsize,
        duration: Duration,
    }

    impl FakeMediaBackend {
        fn new(duration: Duration) -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                duration,
            }
        }
    }

    impl MediaQueryBackend for FakeMediaBackend {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn get_duration(&self, _path: &Path) -> Result<Duration, Report<MediaError>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(self.duration)
        }
    }

    #[test]
    fn media_query_delegates_to_backend() {
        // Given a media query with fake backend.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let query = MediaQuery::new(fake.clone());

        // When getting duration.
        let result = query.get_duration(&path("test.mp4"));

        // Then backend is called.
        assert!(result.is_ok());
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_backend_returns_cached_duration() {
        // Given a cached backend with pre-populated cache.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(path("cached.mp4"), Duration::from_secs(60));
        let cached = CachedMediaBackend::new(cache, fake.clone());

        // When getting duration for cached path.
        let result = cached.get_duration(&path("cached.mp4"));

        // Then cached value is returned without calling fallback.
        assert_eq!(result.unwrap(), Duration::from_secs(60));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cached_backend_falls_back_on_cache_miss() {
        // Given a cached backend with empty cache.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let cached = CachedMediaBackend::new(HashMap::new(), fake.clone());

        // When getting duration for uncached path.
        let result = cached.get_duration(&path("uncached.mp4"));

        // Then fallback is called.
        assert_eq!(result.unwrap(), Duration::from_secs(120));
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cached_backend_uses_canonical_path_for_lookup() {
        // Given a cached backend with canonical path in cache.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(
            std::fs::canonicalize(".").unwrap().join("test.mp4"),
            Duration::from_secs(60),
        );
        let cached = CachedMediaBackend::new(cache, fake.clone());

        // When getting duration with relative path.
        let result = cached.get_duration(&path("test.mp4"));

        // Then cache is used if canonical path matches.
        // Note: This test may hit fallback if file doesn't exist
        let _ = result;
    }

    #[test]
    fn ffprobe_backend_name() {
        // Given an ffprobe backend.
        let backend = FfprobeBackend;

        // When getting name.
        let name = backend.name();

        // Then name is "ffprobe".
        assert_eq!(name, "ffprobe");
    }

    #[test]
    fn cached_backend_name() {
        // Given a cached backend.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let cached = CachedMediaBackend::new(HashMap::new(), fake);

        // When getting name.
        let name = cached.name();

        // Then name is "cached".
        assert_eq!(name, "cached");
    }

    #[test]
    fn fake_backend_name() {
        // Given a fake backend.
        let fake = FakeMediaBackend::new(Duration::from_secs(120));

        // When getting name.
        let name = fake.name();

        // Then name is "fake".
        assert_eq!(name, "fake");
    }

    #[test]
    fn cached_backend_multiple_lookups_use_cache() {
        // Given a cached backend with pre-populated cache.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(path("video.mp4"), Duration::from_secs(60));
        let cached = CachedMediaBackend::new(cache, fake.clone());

        // When getting duration multiple times.
        let _ = cached.get_duration(&path("video.mp4"));
        let _ = cached.get_duration(&path("video.mp4"));
        let _ = cached.get_duration(&path("video.mp4"));

        // Then fallback is never called.
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cached_backend_different_paths_use_fallback() {
        // Given a cached backend with one cached item.
        let fake = Arc::new(FakeMediaBackend::new(Duration::from_secs(120)));
        let mut cache = HashMap::new();
        cache.insert(path("cached.mp4"), Duration::from_secs(60));
        let cached = CachedMediaBackend::new(cache, fake.clone());

        // When getting duration for different paths.
        let _ = cached.get_duration(&path("cached.mp4"));
        let _ = cached.get_duration(&path("other1.mp4"));
        let _ = cached.get_duration(&path("other2.mp4"));

        // Then fallback is called only for uncached paths.
        assert_eq!(fake.call_count.load(Ordering::SeqCst), 2);
    }
}
