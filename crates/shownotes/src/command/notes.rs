use std::path::Path;

use error_stack::{Report, ResultExt};

use crate::feat::{
    external_editor::ExternalEditor, note_db::NoteDb,
    path_resolver::PathResolver, symlink::create_symlink_with_suffix,
};
use crate::services::Services;

use super::CommandError;

pub async fn add(
    services: &Services,
    paths: Vec<std::path::PathBuf>,
) -> Result<Vec<std::path::PathBuf>, Report<CommandError>> {
    if paths.is_empty() {
        return Err(Report::new(CommandError));
    }

    let mut resolved_paths = Vec::with_capacity(paths.len());
    for path in paths {
        let resolved = services
            .path_resolver
            .resolve(&path)
            .await
            .change_context(CommandError)?;
        resolved_paths.push(resolved);
    }

    if resolved_paths.len() == 1 {
        let resolved_path = &resolved_paths[0];
        let path_str = resolved_path.to_string_lossy();
        let file_path_id = services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .change_context(CommandError)?;

        let existing_note = services
            .db
            .get_note(file_path_id)
            .await
            .change_context(CommandError)?;

        let initial_content = existing_note.unwrap_or_default();
        if let Some(new_content) = services
            .editor
            .open(&initial_content)
            .await
            .change_context(CommandError)?
        {
            services
                .db
                .upsert_note(file_path_id, &new_content)
                .await
                .change_context(CommandError)?;
        }
    } else {
        let Some(new_content) = services.editor.open("").await.change_context(CommandError)? else {
            return Ok(resolved_paths);
        };

        for resolved_path in &resolved_paths {
            upsert_note_with_prepend(services, resolved_path, &new_content).await?;
        }
    }

    Ok(resolved_paths)
}

async fn upsert_note_with_prepend(
    services: &Services,
    resolved_path: &Path,
    new_content: &str,
) -> Result<(), Report<CommandError>> {
    let path_str = resolved_path.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(CommandError)?;

    let existing_note = services
        .db
        .get_note(file_path_id)
        .await
        .change_context(CommandError)?;

    let final_content = match existing_note {
        Some(existing) => format!("{existing}\n\n{new_content}"),
        None => new_content.to_owned(),
    };

    services
        .db
        .upsert_note(file_path_id, &final_content)
        .await
        .change_context(CommandError)?;

    Ok(())
}

pub async fn search(
    services: &Services,
    query: &str,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let results: Vec<_> = services
        .db
        .search_notes(query)
        .await
        .change_context(CommandError)?
        .into_iter()
        .collect();

    let mut symlinks_created = 0;
    if create_symlinks {
        let cwd = std::env::current_dir().change_context(CommandError)?;
        for path in &results {
            let src = Path::new(path);
            match create_symlink_with_suffix(src, &cwd) {
                Ok(_) => symlinks_created += 1,
                Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
            }
        }
    }

    Ok((results, symlinks_created))
}

pub async fn fuzzy(
    services: &Services,
    create_symlinks: bool,
) -> Result<(Vec<String>, usize), Report<CommandError>> {
    let notes = services
        .db
        .get_all_notes_with_paths()
        .await
        .change_context(CommandError)?;

    if notes.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let result = services
        .fuzzy_search
        .search(&notes)
        .change_context(CommandError)?;

    let mut symlinks_created = 0;
    if create_symlinks {
        let cwd = std::env::current_dir().change_context(CommandError)?;
        for path in &result.selected_paths {
            let src = Path::new(path);
            match create_symlink_with_suffix(src, &cwd) {
                Ok(_) => symlinks_created += 1,
                Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
            }
        }
    }

    Ok((result.selected_paths, symlinks_created))
}
