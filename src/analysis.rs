use std::{collections::HashMap, io::Write, path::PathBuf};

use error_stack::Report;

use crate::media::{MediaError, MediaQueryBackend};
use crate::playlist::FileMetadata;

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
