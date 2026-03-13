use error_stack::{Report, ResultExt};

use marked_path::CanonicalPath;

use crate::system_ctx::SystemCtx;
use crate::feat::sources::SourceDb;
use crate::feat::note_db::NoteDb;
use crate::feat::external_editor::ExternalEditor;

use super::CommandError;

pub async fn add(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    url: &str,
) -> Result<(), Report<CommandError>> {
    let path_str = path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let mut existing = ctx
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?
        .into_iter()
        .map(|s| s.source_url)
        .collect::<Vec<_>>();
    existing.push(url.to_string());

    ctx
        .services
        .sources
        .set_sources(file_path_id, &existing)
        .await
        .change_context(CommandError)?;

    Ok(())
}

pub async fn list(
    ctx: &SystemCtx,
    path: &CanonicalPath,
) -> Result<Vec<String>, Report<CommandError>> {
    let path_str = path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let sources = ctx
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?;

    let urls = sources.into_iter().map(|s| s.source_url).collect();
    Ok(urls)
}

pub async fn edit(
    ctx: &SystemCtx,
    path: &CanonicalPath,
) -> Result<(), Report<CommandError>> {
    let path_str = path.as_path().to_string_lossy();
    let file_path_id = ctx
        .services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let existing = ctx
        .services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(CommandError)?;

    let initial_content = existing
        .iter()
        .map(|s| s.source_url.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if let Some(new_content) = ctx
        .services
        .editor
        .open(&initial_content)
        .await
        .change_context(CommandError)?
    {
        let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
        ctx
            .services
            .sources
            .set_sources(file_path_id, &urls)
            .await
            .change_context(CommandError)?;
    }

    Ok(())
}
