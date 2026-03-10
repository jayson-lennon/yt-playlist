use std::path::Path;

use error_stack::{Report, ResultExt};

use crate::feat::create_symlink_with_suffix;
use crate::feat::external_editor::ExternalEditor;
use crate::feat::note_db::NoteDb;
use crate::feat::path_resolver::PathResolver;
use crate::services::Services;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct AddNoteError;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct FuzzyNotesError;

pub struct FuzzyResult {
    pub paths: Vec<String>,
    pub symlinks_created: usize,
}

pub async fn add_note(services: &Services, path: &Path) -> Result<(), Report<AddNoteError>> {
    let resolved = services
        .path_resolver
        .resolve(path)
        .await
        .change_context(AddNoteError)?;

    let path_str = resolved.to_string_lossy();
    let file_path_id = services
        .db
        .get_or_create_file_path(&path_str)
        .await
        .change_context(AddNoteError)?;

    let existing_note = services
        .db
        .get_note(file_path_id)
        .await
        .change_context(AddNoteError)?;

    let initial_content = existing_note.unwrap_or_default();
    if let Some(new_content) = services
        .editor
        .open(&initial_content)
        .await
        .change_context(AddNoteError)?
    {
        services
            .db
            .upsert_note(file_path_id, &new_content)
            .await
            .change_context(AddNoteError)?;
    }

    Ok(())
}

pub async fn fuzzy_notes(
    services: &Services,
    create_symlinks: bool,
) -> Result<FuzzyResult, Report<FuzzyNotesError>> {
    let notes = services
        .db
        .get_all_notes_with_paths()
        .await
        .change_context(FuzzyNotesError)?;

    if notes.is_empty() {
        return Ok(FuzzyResult {
            paths: Vec::new(),
            symlinks_created: 0,
        });
    }

    let result = services
        .fuzzy_search
        .search(&notes)
        .change_context(FuzzyNotesError)?;

    let mut symlinks_created = 0;
    if create_symlinks {
        let cwd = std::env::current_dir().change_context(FuzzyNotesError)?;
        for path in &result.selected_paths {
            let src = Path::new(path);
            match create_symlink_with_suffix(src, &cwd) {
                Ok(_) => symlinks_created += 1,
                Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
            }
        }
    }

    Ok(FuzzyResult {
        paths: result.selected_paths,
        symlinks_created,
    })
}
