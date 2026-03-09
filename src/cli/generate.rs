use std::{path::Path, sync::Arc};

use error_stack::{Report, ResultExt};

use crate::{
    feat::generate_show_notes,
    notes::SystemServicesHandle,
    playlist::{PlaylistStorage, PlaylistStorageBackend, TomlBackend},
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

        let output = generate_show_notes(&playlist_data, &services.sources, format)
            .await
            .change_context(RunError)?;

        println!("{output}");
        Ok(())
    })
}
