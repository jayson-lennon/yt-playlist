use std::sync::Arc;

use error_stack::{Report, ResultExt};

use crate::feat::playlist::{PlaylistStorage, PlaylistStorageService, TomlStorage};
use crate::services::Services;

use super::CommandError;

pub async fn execute(
    services: &Services,
    playlist_path: &std::path::Path,
    format: &str,
) -> Result<String, Report<CommandError>> {
    let storage_backend: Arc<dyn PlaylistStorage> =
        Arc::new(TomlStorage::new(playlist_path.to_path_buf()));
    let playlist_storage = PlaylistStorageService::new(storage_backend);
    let playlist_data = playlist_storage.load().change_context(CommandError)?;

    crate::feat::generate_show_notes(&playlist_data, &services.sources, format)
        .await
        .change_context(CommandError)
}
