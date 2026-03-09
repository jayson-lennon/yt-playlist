use error_stack::{Report, ResultExt};
use crate::services::Services;
use crate::feat::path_resolver::PathResolver;
use crate::feat::note_db::NoteDb;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct SourcesError;

pub async fn resolve_and_get_file_path(
    services: &Services,
    path: &std::path::Path,
) -> Result<(std::path::PathBuf, i64), Report<SourcesError>> {
    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(SourcesError)?;

    let path_str = resolved.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(SourcesError)?;

    Ok((resolved, file_path_id))
}
