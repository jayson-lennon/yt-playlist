use std::{path::Path, sync::Arc};

use error_stack::{Report, ResultExt};

use crate::{
    format::FormatRegistry,
    notes::SystemServicesHandle,
    playlist::{PlaylistStorage, PlaylistStorageBackend, TomlBackend},
    sources::SourceDb,
};

use super::RunError;

/// Generates show notes from a playlist.
///
/// # Errors
///
/// Returns an error if:
/// - The playlist cannot be loaded
/// - The database cannot be accessed
/// - The format is not recognized
pub fn run_generate(
    format: &str,
    playlist_path: &Path,
    db_path: &Path,
) -> Result<(), Report<RunError>> {
    let storage_backend: Arc<dyn PlaylistStorageBackend> =
        Arc::new(TomlBackend::new(playlist_path.to_path_buf()));
    let playlist_storage = PlaylistStorage::new(storage_backend);
    let playlist_data = playlist_storage.load().change_context(RunError)?;

    let rt = tokio::runtime::Runtime::new().change_context(RunError)?;
    rt.block_on(async {
        let services = SystemServicesHandle::new(&db_path.to_string_lossy())
            .await
            .change_context(RunError)?;

        let paths: Vec<String> = playlist_data
            .playlist
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        let sources_map = services
            .sources
            .get_sources_for_paths(&paths)
            .await
            .change_context(RunError)?;

        let registry = FormatRegistry::new();
        let formatter = registry
            .get(format)
            .ok_or_else(|| Report::new(RunError))?;

        let entries: Vec<crate::format::ShowNotesEntry> = playlist_data
            .playlist
            .iter()
            .filter_map(|path| {
                let path_str = path.to_string_lossy();
                let filename = path.file_name().map_or_else(
                    || path_str.clone().into_owned(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let alias = playlist_data
                    .files
                    .get(path)
                    .and_then(|m| m.alias.clone());
                let sources: Vec<String> = sources_map
                    .get(&*path_str)
                    .map(|v| v.iter().map(|s| s.source_url.clone()).collect())
                    .unwrap_or_default();

                if sources.is_empty() {
                    None
                } else {
                    Some(crate::format::ShowNotesEntry {
                        path: path_str.into_owned(),
                        filename,
                        alias,
                        sources,
                    })
                }
            })
            .collect();

        println!("{}", formatter.format(&entries));
        Ok(())
    })
}
