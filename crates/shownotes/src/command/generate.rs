use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::services::Services;

use super::CommandError;

pub async fn execute(
    services: &Services,
    playlist_path: &std::path::Path,
    format: &str,
) -> Result<String, Report<CommandError>> {
    let working_directory = playlist_path.parent().unwrap_or(playlist_path);
    let working_directory = CanonicalPath::from_path(working_directory)
        .map_err(|_| Report::new(CommandError))
        .attach("Failed to canonicalize working directory")?;
    let playlist_data = services.storage.load(&working_directory).await.change_context(CommandError)?;

    crate::feat::generate_show_notes(&playlist_data, &services.sources, format)
        .await
        .change_context(CommandError)
}
