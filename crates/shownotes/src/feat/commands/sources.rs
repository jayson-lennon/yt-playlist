use std::path::Path;
use error_stack::{Report, ResultExt};
use crate::services::Services;
use crate::feat::sources::SourceDb;
use crate::feat::external_editor::ExternalEditor;
use crate::feat::path_resolver::PathResolver;
use crate::feat::note_db::NoteDb;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct EditSourcesError;

pub async fn edit_sources(
    services: &Services,
    path: &Path,
) -> Result<(), Report<EditSourcesError>> {
    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(EditSourcesError)?;

    let path_str = resolved.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(EditSourcesError)?;

    let existing = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(EditSourcesError)?;

    let initial_content = existing
        .iter()
        .map(|s| s.source_url.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(new_content) = services
        .editor
        .open(&initial_content)
        .await
        .change_context(EditSourcesError)?
    {
        let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
        services
            .sources
            .set_sources(file_path_id, &urls)
            .await
            .change_context(EditSourcesError)?;
    }

    Ok(())
}
