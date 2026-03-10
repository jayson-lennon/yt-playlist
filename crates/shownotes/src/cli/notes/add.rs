use std::path::{Path, PathBuf};

use error_stack::{Report, ResultExt};

use crate::feat::external_editor::ExternalEditor;
use crate::feat::note_db::NoteDb;
use crate::feat::path_resolver::PathResolver;
use crate::services::Services;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct AddError;

pub async fn handle_add_command(
    services: &Services,
    paths: Vec<PathBuf>,
) -> Result<(), Report<AddError>> {
    if paths.is_empty() {
        return Err(Report::new(AddError));
    }

    let resolved_paths = resolve_all_paths(services, paths).await?;

    if resolved_paths.len() == 1 {
        handle_single_path(services, &resolved_paths[0]).await?;
    } else {
        handle_multiple_paths(services, resolved_paths).await?;
    }

    Ok(())
}

async fn resolve_all_paths(
    services: &Services,
    paths: Vec<PathBuf>,
) -> Result<Vec<PathBuf>, Report<AddError>> {
    let mut resolved_paths = Vec::with_capacity(paths.len());
    for path in paths {
        let resolved = services
            .path_resolver
            .resolve(&path)
            .await
            .change_context(AddError)?;
        resolved_paths.push(resolved);
    }
    Ok(resolved_paths)
}

async fn handle_single_path(
    services: &Services,
    resolved_path: &Path,
) -> Result<(), Report<AddError>> {
    let path_str = resolved_path.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(AddError)?;

    let existing_note = services
        .db
        .get_note(file_path_id)
        .await
        .change_context(AddError)?;

    let initial_content = existing_note.unwrap_or_default();
    if let Some(new_content) = services
        .editor
        .open(&initial_content)
        .await
        .change_context(AddError)?
    {
        services
            .db
            .upsert_note(file_path_id, &new_content)
            .await
            .change_context(AddError)?;
    }

    Ok(())
}

async fn handle_multiple_paths(
    services: &Services,
    resolved_paths: Vec<PathBuf>,
) -> Result<(), Report<AddError>> {
    let Some(new_content) = services.editor.open("").await.change_context(AddError)? else {
        return Ok(());
    };

    for resolved_path in resolved_paths {
        upsert_note_with_prepend(services, &resolved_path, &new_content).await?;
    }

    Ok(())
}

async fn upsert_note_with_prepend(
    services: &Services,
    resolved_path: &Path,
    new_content: &str,
) -> Result<(), Report<AddError>> {
    let path_str = resolved_path.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(AddError)?;

    let existing_note = services
        .db
        .get_note(file_path_id)
        .await
        .change_context(AddError)?;

    let final_content = match existing_note {
        Some(existing) => format!("{existing}\n\n{new_content}"),
        None => new_content.to_owned(),
    };

    services
        .db
        .upsert_note(file_path_id, &final_content)
        .await
        .change_context(AddError)?;

    Ok(())
}
