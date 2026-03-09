use std::{
    path::PathBuf,
    sync::Arc,
};

use clap::Subcommand;
use error_stack::{Report, ResultExt};

use crate::feat::fuzzy_search::{FuzzySearchService, backend::SkimBackend};
use crate::feat::{ExternalEditor, NoteDb, PathResolver, create_symlink_with_suffix};
use crate::services::Services;

use super::RunError;

#[derive(Subcommand)]
pub enum NotesCommands {
    /// Add notes to files
    Add { paths: Vec<PathBuf> },

    /// Search for words in notes
    Search {
        /// Search query. Uses AND matching.
        query: String,
        /// Create symlinks to located results in current directory.
        #[arg(long)]
        symlink: bool,
    },

    /// Fuzzy search through all notes
    Fuzzy {
        /// Create symlinks to located results in current directory.
        #[arg(long)]
        symlink: bool,
    },
}

/// Runs a notes command.
///
/// # Errors
///
/// Returns an error if:
/// - The database cannot be accessed
/// - Path resolution fails
/// - The editor fails to open
/// - The fuzzy search process fails to spawn or communicate
pub fn run_notes_command(
    cmd: NotesCommands,
    db_path: &std::path::Path,
    rt: &tokio::runtime::Handle,
) -> Result<(), Report<RunError>> {
    rt.block_on(async { run_notes_command_async(cmd, db_path, rt).await })
}

#[allow(clippy::too_many_lines)]
async fn run_notes_command_async(
    cmd: NotesCommands,
    db_path: &std::path::Path,
    rt: &tokio::runtime::Handle,
) -> Result<(), Report<RunError>> {
    let services = Services::new(&db_path.to_string_lossy(), rt.clone())
        .await
        .change_context(RunError)?;

    match cmd {
        NotesCommands::Add { paths } => {
            if paths.is_empty() {
                return Err(Report::new(RunError));
            }

            let mut resolved_paths = Vec::with_capacity(paths.len());
            for path in paths {
                let resolved = services
                    .path_resolver
                    .resolve(&path)
                    .await
                    .change_context(RunError)?;
                resolved_paths.push(resolved);
            }

            if resolved_paths.len() == 1 {
                let resolved_path = &resolved_paths[0];
                let path_str = resolved_path.to_string_lossy();
                let file_path_id = services
                    .db
                    .get_or_create_file_path(&path_str)
                    .await
                    .change_context(RunError)?;

                let existing_note = services
                    .db
                    .get_note(file_path_id)
                    .await
                    .change_context(RunError)?;

                let initial_content = existing_note.unwrap_or_default();
                if let Some(new_content) = services
                    .editor
                    .open(&initial_content)
                    .await
                    .change_context(RunError)?
                {
                    services
                        .db
                        .upsert_note(file_path_id, &new_content)
                        .await
                        .change_context(RunError)?;
                }
            } else if let Some(new_content) =
                services.editor.open("").await.change_context(RunError)?
            {
                for resolved_path in resolved_paths {
                    let path_str = resolved_path.to_string_lossy();
                    let file_path_id = services
                        .db
                        .get_or_create_file_path(&path_str)
                        .await
                        .change_context(RunError)?;

                    let existing_note = services
                        .db
                        .get_note(file_path_id)
                        .await
                        .change_context(RunError)?;

                    let final_content = match existing_note {
                        Some(existing) => format!("{existing}\n\n{new_content}"),
                        None => new_content.clone(),
                    };

                    services
                        .db
                        .upsert_note(file_path_id, &final_content)
                        .await
                        .change_context(RunError)?;
                }
            }
        }
        NotesCommands::Search { query, symlink } => {
            let results = services
                .db
                .search_notes(&query)
                .await
                .change_context(RunError)?;

            let cwd = std::env::current_dir().change_context(RunError)?;

            for path in &results {
                println!("{path}");
            }

            if symlink {
                for path in &results {
                    let src = PathBuf::from(path);
                    match create_symlink_with_suffix(&src, &cwd) {
                        Ok(dest) => eprintln!("Created symlink: {}", dest.display()),
                        Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
                    }
                }
            }
        }
        NotesCommands::Fuzzy { symlink } => {
            let fuzzy_search = FuzzySearchService::new(Arc::new(SkimBackend));
            
            let notes = services
                .db
                .get_all_notes_with_paths()
                .await
                .change_context(RunError)?;

            if notes.is_empty() {
                return Ok(());
            }

            let result = fuzzy_search
                .search(&notes)
                .change_context(RunError)?;

            for path in &result.selected_paths {
                println!("{path}");
            }

            if symlink {
                let cwd = std::env::current_dir().change_context(RunError)?;
                for path in &result.selected_paths {
                    let src = PathBuf::from(path);
                    match create_symlink_with_suffix(&src, &cwd) {
                        Ok(dest) => eprintln!("Created symlink: {}", dest.display()),
                        Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
                    }
                }
            }
        }
    }

    Ok(())
}
