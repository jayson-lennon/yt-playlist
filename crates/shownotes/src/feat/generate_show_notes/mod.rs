pub mod format;

use std::collections::HashMap;

use error_stack::{Report, ResultExt};
use wherror::Error;

use crate::{
    feat::generate_show_notes::format::{FormatRegistry, ShowNotesEntry},
    feat::playlist::PlaylistData,
    feat::sources::{Source, SourceDb},
};

#[derive(Debug, Error)]
#[error(debug)]
pub struct GenerateShowNotesError(pub String);

/// Generates show notes for a playlist in the specified format.
///
/// # Errors
///
/// Returns an error if:
/// - Fetching sources from the database fails
/// - The specified format is not recognized
pub async fn generate_show_notes(
    playlist_data: &PlaylistData,
    sources: &dyn SourceDb,
    format: &str,
) -> Result<String, Report<GenerateShowNotesError>> {
    let paths: Vec<String> = playlist_data
        .playlist
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    let sources_map =
        sources
            .get_sources_for_paths(&paths)
            .await
            .change_context(GenerateShowNotesError(
                "Failed to fetch sources".to_string(),
            ))?;

    let registry = FormatRegistry::new();
    let formatter = registry
        .get(format)
        .ok_or_else(|| Report::new(GenerateShowNotesError(format!("Unknown format: {format}"))))?;

    let entries = build_entries(playlist_data, &sources_map);
    Ok(formatter.format(&entries))
}

fn build_entries(
    playlist_data: &PlaylistData,
    sources_map: &HashMap<String, Vec<Source>>,
) -> Vec<ShowNotesEntry> {
    playlist_data
        .playlist
        .iter()
        .filter_map(|path| {
            let path_str = path.to_string_lossy();
            let filename = path
                .file_stem()
                .map(str::to_string)
                .unwrap_or_default();
            let sources: Vec<String> = sources_map
                .get(&*path_str)
                .map(|v| v.iter().map(|s| s.source_url.clone()).collect())
                .unwrap_or_default();

            if sources.is_empty() {
                None
            } else {
                Some(ShowNotesEntry {
                    path: path_str.into_owned(),
                    filename,
                    alias: None,
                    sources,
                })
            }
        })
        .collect()
}
