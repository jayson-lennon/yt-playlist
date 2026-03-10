use std::path::PathBuf;
use error_stack::{Report, ResultExt};
use crate::services::Services;
use crate::feat::sources::SourceDb;
use crate::feat::external_editor::ExternalEditor;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct EditError;

pub async fn handle_edit_command(
    services: &Services,
    path: PathBuf,
) -> Result<(), Report<EditError>> {
    let (resolved, file_path_id) = super::common::resolve_and_get_file_path(services, &path)
        .await
        .change_context(EditError)?;

    let existing = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(EditError)?;

    let initial_content = existing
        .iter()
        .map(|s| s.source_url.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(new_content) = services
        .editor
        .open(&initial_content)
        .await
        .change_context(EditError)?
    {
        let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
        services
            .sources
            .set_sources(file_path_id, &urls)
            .await
            .change_context(EditError)?;
        println!("Updated sources for: {}", resolved.display());
    }

    Ok(())
}
