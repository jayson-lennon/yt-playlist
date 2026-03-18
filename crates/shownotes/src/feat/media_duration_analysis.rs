use std::{collections::HashMap, io::Write};

use error_stack::Report;
use marked_path::CanonicalPath;

use super::media_query::{MediaError, MediaQuery};
use super::playlist::FileMetadata;

/// Result of media duration analysis.
///
/// Contains a mapping from canonical file paths to their metadata,
/// including resolved durations for media files.
pub struct AnalysisResult {
    /// Map of canonical file paths to their metadata.
    pub files: HashMap<CanonicalPath, FileMetadata>,
}

#[allow(clippy::missing_errors_doc, clippy::implicit_hasher)]
pub fn analyze_files(
    files: &[CanonicalPath],
    mut metadata: HashMap<CanonicalPath, FileMetadata>,
    backend: &dyn MediaQuery,
    silent: bool,
) -> Result<AnalysisResult, Report<MediaError>> {
    let uncached: Vec<_> = files
        .iter()
        .filter(|p| {
            !metadata.contains_key(*p) || metadata.get(*p).and_then(|m| m.duration).is_none()
        })
        .collect();

    let total = uncached.len();
    if total > 0 {
        if !silent {
            eprint!("Analyzing durations: 0/{total}");
            std::io::stderr().flush().ok();
        }

        for (i, path) in uncached.iter().enumerate() {
            if let Ok(duration) = backend.get_duration(path.as_path()) {
                let existing = metadata.remove(*path);
                let time_added = existing.as_ref().and_then(|m| m.time_added);
                let alias = existing.and_then(|m| m.alias);
                metadata.insert(
                    (*path).clone(),
                    FileMetadata {
                        duration: Some(duration),
                        is_virtual: false,
                        deleted: false,
                        mime_type: None,
                        time_added,
                        alias,
                    },
                );
            }
            if !silent {
                eprint!("\rAnalyzing durations: {}/{}", i + 1, total);
                std::io::stderr().flush().ok();
            }
        }
        if !silent {
            eprintln!();
        }
    }

    Ok(AnalysisResult { files: metadata })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    struct FakeMediaBackend {
        durations: HashMap<PathBuf, Duration>,
    }

    impl FakeMediaBackend {
        fn new() -> Self {
            Self {
                durations: HashMap::new(),
            }
        }

        fn with_duration(mut self, p: &Path, duration: Duration) -> Self {
            self.durations.insert(p.to_path_buf(), duration);
            self
        }
    }

    impl MediaQuery for FakeMediaBackend {
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

    fn create_temp_files(temp: &TempDir, names: &[&str]) -> Vec<CanonicalPath> {
        names
            .iter()
            .map(|name| {
                let path = temp.path().join(name);
                std::fs::write(&path, "").unwrap();
                CanonicalPath::from_path(&path).unwrap()
            })
            .collect()
    }

    #[test]
    fn analyze_files_processes_uncached_files() {
        let temp = TempDir::new().unwrap();
        let files = create_temp_files(&temp, &["a.mp4", "b.mp4"]);
        let metadata = HashMap::new();
        let backend = FakeMediaBackend::new()
            .with_duration(files[0].as_path(), Duration::from_secs(120))
            .with_duration(files[1].as_path(), Duration::from_secs(60));

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        assert_eq!(result.files.len(), 2);
        assert_eq!(
            result.files.get(&files[0]).unwrap().duration,
            Some(Duration::from_secs(120))
        );
        assert_eq!(
            result.files.get(&files[1]).unwrap().duration,
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn analyze_files_skips_cached_files() {
        let temp = TempDir::new().unwrap();
        let files = create_temp_files(&temp, &["a.mp4", "b.mp4"]);
        let mut metadata = HashMap::new();
        metadata.insert(
            files[0].clone(),
            FileMetadata {
                duration: Some(Duration::from_secs(100)),
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );
        let backend =
            FakeMediaBackend::new().with_duration(files[1].as_path(), Duration::from_secs(60));

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        assert_eq!(
            result.files.get(&files[0]).unwrap().duration,
            Some(Duration::from_secs(100))
        );
    }

    #[test]
    fn analyze_files_processes_files_with_missing_duration() {
        let temp = TempDir::new().unwrap();
        let files = create_temp_files(&temp, &["a.mp4"]);
        let mut metadata = HashMap::new();
        metadata.insert(
            files[0].clone(),
            FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );
        let backend =
            FakeMediaBackend::new().with_duration(files[0].as_path(), Duration::from_secs(120));

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        let meta = result.files.get(&files[0]).unwrap();
        assert_eq!(meta.duration, Some(Duration::from_secs(120)));
    }

    #[test]
    fn analyze_files_preserves_existing_time_added() {
        let temp = TempDir::new().unwrap();
        let files = create_temp_files(&temp, &["a.mp4"]);
        let timestamp = "2024-01-01T00:00:00Z".parse().unwrap();
        let mut metadata = HashMap::new();
        metadata.insert(
            files[0].clone(),
            FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: Some(timestamp),
                alias: None,
            },
        );
        let backend =
            FakeMediaBackend::new().with_duration(files[0].as_path(), Duration::from_secs(120));

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        assert_eq!(
            result.files.get(&files[0]).unwrap().time_added,
            Some(timestamp)
        );
    }

    #[test]
    fn analyze_files_handles_empty_file_list() {
        let files: Vec<CanonicalPath> = vec![];
        let metadata = HashMap::new();
        let backend = FakeMediaBackend::new();

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        assert!(result.files.is_empty());
    }

    #[test]
    fn analyze_files_handles_backend_errors() {
        let temp = TempDir::new().unwrap();
        let files = create_temp_files(&temp, &["a.mp4", "b.mp4"]);
        let metadata = HashMap::new();
        let backend =
            FakeMediaBackend::new().with_duration(files[0].as_path(), Duration::from_secs(120));

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        assert_eq!(result.files.len(), 1);
        assert!(result.files.contains_key(&files[0]));
        assert!(!result.files.contains_key(&files[1]));
    }

    #[test]
    fn analyze_files_preserves_existing_metadata() {
        let temp = TempDir::new().unwrap();
        let files = create_temp_files(&temp, &["a.mp4", "b.mp4"]);
        let mut metadata = HashMap::new();
        metadata.insert(
            files[0].clone(),
            FileMetadata {
                duration: Some(Duration::from_secs(100)),
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );
        metadata.insert(
            files[1].clone(),
            FileMetadata {
                duration: None,
                is_virtual: false,
                deleted: false,
                mime_type: None,
                time_added: None,
                alias: None,
            },
        );
        let backend =
            FakeMediaBackend::new().with_duration(files[1].as_path(), Duration::from_secs(60));

        let result = analyze_files(&files, metadata, &backend, true).unwrap();

        let meta_a = result.files.get(&files[0]).unwrap();
        assert_eq!(meta_a.duration, Some(Duration::from_secs(100)));

        let meta_b = result.files.get(&files[1]).unwrap();
        assert_eq!(meta_b.duration, Some(Duration::from_secs(60)));
    }
}
