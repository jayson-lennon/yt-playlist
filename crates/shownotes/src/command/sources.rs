use std::path::Path;

use error_stack::{Report, ResultExt};

use crate::services::Services;
use crate::feat::path_resolver::PathResolver;
use crate::feat::sources::SourceDb;
use crate::feat::note_db::NoteDb;
use crate::feat::external_editor::ExternalEditor;

use super::CommandError;

pub async fn add(
    services: &Services,
    path: &Path,
    url: &str,
) -> Result<(), Report<CommandError>> {
    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(CommandError)?;

    let path_str = resolved.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let mut existing = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?
        .into_iter()
        .map(|s| s.source_url)
        .collect::<Vec<_>>();
    existing.push(url.to_string());

    services
        .sources
        .set_sources(file_path_id, &existing)
        .await
        .change_context(CommandError)?;

    Ok(())
}

pub async fn list(
    services: &Services,
    path: &Path,
) -> Result<Vec<String>, Report<CommandError>> {
    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(CommandError)?;

    let path_str = resolved.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let sources = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?;

    let urls = sources.into_iter().map(|s| s.source_url).collect();
    Ok(urls)
}

pub async fn edit(
    services: &Services,
    path: &Path,
) -> Result<(), Report<CommandError>> {
    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(CommandError)?;

    let path_str = resolved.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let existing = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?;

    let initial_content = existing
        .iter()
        .map(|s| s.source_url.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(new_content) = services
        .editor
        .open(&initial_content)
        .await
        .change_context(CommandError)?
    {
        let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
        services
            .sources
            .set_sources(file_path_id, &urls)
            .await
            .change_context(CommandError)?;
    }

    Ok(())
}
