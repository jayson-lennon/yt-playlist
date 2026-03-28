// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod format;

pub use format::{FormatRegistry, ShowNotesEntry, ShowNotesFormat};

use std::collections::HashMap;

use error_stack::{Report, ResultExt};
use wherror::Error;

use crate::{
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
