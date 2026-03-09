use std::{collections::HashMap, io::Write, path::PathBuf};

use error_stack::Report;

use super::media_query::{MediaError, MediaQueryBackend};
use super::playlist::FileMetadata;

pub struct AnalysisResult {
    pub files: HashMap<PathBuf, FileMetadata>,
}

#[allow(clippy::missing_errors_doc, clippy::implicit_hasher)]
pub fn analyze_files(
    files: &[PathBuf],
    mut metadata: HashMap<PathBuf, FileMetadata>,
    backend: &dyn MediaQueryBackend,
) -> Result<AnalysisResult, Report<MediaError>> {
    let uncached: Vec<_> = files
        .iter()
        .filter(|p| {
            !metadata.contains_key(*p) || metadata.get(*p).and_then(|m| m.duration).is_none()
        })
        .collect();

    let total = uncached.len();
    if total > 0 {
        eprint!("Analyzing durations: 0/{total}");
        std::io::stderr().flush().ok();

        for (i, path) in uncached.iter().enumerate() {
            if let Ok(duration) = backend.get_duration(path) {
                let existing = metadata.remove(*path);
                let alias = existing.and_then(|m| m.alias);
                metadata.insert(
                    (*path).clone(),
                    FileMetadata {
                        duration: Some(duration),
                        alias,
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                    },
                );
            }
            eprint!("\rAnalyzing durations: {}/{}", i + 1, total);
            std::io::stderr().flush().ok();
        }
        eprintln!();
    }

    Ok(AnalysisResult { files: metadata })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;

    fn path(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    struct FakeMediaBackend {
        durations: HashMap<PathBuf, Duration>,
    }

    impl FakeMediaBackend {
        fn new() -> Self {
            Self {
                durations: HashMap::new(),
            }
        }

        fn with_duration(mut self, p: &str, duration: Duration) -> Self {
            self.durations.insert(path(p), duration);
            self
        }
    }

    impl MediaQueryBackend for FakeMediaBackend {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
            self.durations
                .get(path)
                .copied()
                .ok_or_else(|| Report::new(MediaError))
        }
    }

    #[test]
    fn analyze_files_processes_uncached_files() {
        // Given files without cached metadata.
        let files = vec![path("a.mp4"), path("b.mp4")];
        let metadata = HashMap::new();
        let backend = FakeMediaBackend::new()
            .with_duration("a.mp4", Duration::from_secs(120))
            .with_duration("b.mp4", Duration::from_secs(60));

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then durations are added.
        assert_eq!(result.files.len(), 2);
        assert_eq!(
            result.files.get(&path("a.mp4")).unwrap().duration,
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            result.files.get(&path("b.mp4")).unwrap().duration,
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn analyze_files_skips_cached_files() {
        // Given files with some already cached.
        let files = vec![path("a.mp4"), path("b.mp4")];
        let mut metadata = HashMap::new();
        metadata.insert(
            path("a.mp4"),
            FileMetadata {
                duration: Some(Duration::from_secs(100)),
                alias: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        let backend = FakeMediaBackend::new().with_duration("b.mp4", Duration::from_secs(60));

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then cached duration is preserved.
        assert_eq!(
            result.files.get(&path("a.mp4")).unwrap().duration,
            Some(Duration::from_secs(100))
        );
    }

    #[test]
    fn analyze_files_processes_files_with_missing_duration() {
        // Given files with metadata but no duration.
        let files = vec![path("a.mp4")];
        let mut metadata = HashMap::new();
        metadata.insert(
            path("a.mp4"),
            FileMetadata {
                duration: None,
                alias: Some("My Video".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        let backend = FakeMediaBackend::new().with_duration("a.mp4", Duration::from_secs(120));

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then duration is added and alias is preserved.
        let meta = result.files.get(&path("a.mp4")).unwrap();
        assert_eq!(meta.duration, Some(Duration::from_secs(120)));
        assert_eq!(meta.alias, Some("My Video".to_string()));
    }

    #[test]
    fn analyze_files_preserves_existing_aliases() {
        // Given files with aliases but no durations.
        let files = vec![path("a.mp4")];
        let mut metadata = HashMap::new();
        metadata.insert(
            path("a.mp4"),
            FileMetadata {
                duration: None,
                alias: Some("My Video".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        let backend = FakeMediaBackend::new().with_duration("a.mp4", Duration::from_secs(120));

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then alias is preserved.
        assert_eq!(
            result.files.get(&path("a.mp4")).unwrap().alias,
            Some("My Video".to_string())
        );
    }

    #[test]
    fn analyze_files_handles_empty_file_list() {
        // Given no files.
        let files: Vec<PathBuf> = vec![];
        let metadata = HashMap::new();
        let backend = FakeMediaBackend::new();

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then result is empty.
        assert!(result.files.is_empty());
    }

    #[test]
    fn analyze_files_handles_backend_errors() {
        // Given files where backend will fail.
        let files = vec![path("a.mp4"), path("b.mp4")];
        let metadata = HashMap::new();
        let backend = FakeMediaBackend::new().with_duration("a.mp4", Duration::from_secs(120));

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then only successful files are added.
        assert_eq!(result.files.len(), 1);
        assert!(result.files.contains_key(&path("a.mp4")));
        assert!(!result.files.contains_key(&path("b.mp4")));
    }

    #[test]
    fn analyze_files_preserves_existing_metadata() {
        // Given files with existing metadata.
        let files = vec![path("a.mp4"), path("b.mp4")];
        let mut metadata = HashMap::new();
        metadata.insert(
            path("a.mp4"),
            FileMetadata {
                duration: Some(Duration::from_secs(100)),
                alias: Some("Video A".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        metadata.insert(
            path("b.mp4"),
            FileMetadata {
                duration: None,
                alias: Some("Video B".to_string()),
                is_virtual: false,
                deleted: false,
                mime_type: None,
            },
        );
        let backend = FakeMediaBackend::new().with_duration("b.mp4", Duration::from_secs(60));

        // When analyzing.
        let result = analyze_files(&files, metadata, &backend).unwrap();

        // Then all metadata is preserved/updated.
        let meta_a = result.files.get(&path("a.mp4")).unwrap();
        assert_eq!(meta_a.duration, Some(Duration::from_secs(100)));
        assert_eq!(meta_a.alias, Some("Video A".to_string()));

        let meta_b = result.files.get(&path("b.mp4")).unwrap();
        assert_eq!(meta_b.duration, Some(Duration::from_secs(60)));
        assert_eq!(meta_b.alias, Some("Video B".to_string()));
    }
}
