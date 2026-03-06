use std::{collections::HashMap, io::Write, path::PathBuf, time::Duration};

use error_stack::Report;

use crate::cache::DurationCache;
use crate::media::{MediaError, MediaQueryBackend};

pub struct AnalysisResult {
    pub durations: HashMap<PathBuf, Duration>,
    pub cache: DurationCache,
}

#[allow(clippy::missing_errors_doc)]
pub fn analyze_files(
    files: &[PathBuf],
    mut cache: DurationCache,
    backend: &dyn MediaQueryBackend,
) -> Result<AnalysisResult, Report<MediaError>> {
    let uncached: Vec<_> = files.iter().filter(|p| !cache.contains(p)).collect();

    let total = uncached.len();
    if total > 0 {
        eprint!("Analyzing durations: 0/{total}");
        std::io::stderr().flush().ok();

        for (i, path) in uncached.iter().enumerate() {
            if let Ok(duration) = backend.get_duration(path) {
                cache.insert((*path).clone(), duration);
            }
            eprint!("\rAnalyzing durations: {}/{}", i + 1, total);
            std::io::stderr().flush().ok();
        }
        eprintln!();

        if let Err(e) = cache.save() {
            eprintln!("Warning: failed to save cache: {e:?}");
        }
    }

    let durations: HashMap<PathBuf, Duration> = files
        .iter()
        .filter_map(|p| cache.get(p).map(|d| (p.clone(), d)))
        .collect();

    Ok(AnalysisResult { durations, cache })
}
