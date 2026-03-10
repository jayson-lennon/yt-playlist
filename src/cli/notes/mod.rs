mod add;
mod fuzzy;
mod search;
mod symlink;

use std::path::PathBuf;

use clap::Subcommand;
use error_stack::{Report, ResultExt};

use crate::services::Services;

use super::RunError;

#[derive(Subcommand)]
pub enum NotesCommand {
    /// Add or edit notes for one or more files
    ///
    /// Opens an external editor to create or modify notes associated with the
    /// specified file paths. For a single file, edits the existing note (if any).
    /// For multiple files, the entered content is prepended to each file's note.
    Add {
        /// File paths to add notes for
        paths: Vec<PathBuf>,
    },
    /// Search notes by query string
    ///
    /// Searches all stored notes for the given query and prints matching file paths.
    /// Optionally creates symlinks to matched files in the current directory.
    Search {
        /// Search query string
        query: String,
        /// Create symlinks to matched files in current directory
        #[arg(long)]
        symlink: bool,
    },
    /// Interactive fuzzy search through all notes
    ///
    /// Opens an interactive fuzzy finder to search through all stored notes.
    /// Prints selected file paths and optionally creates symlinks in the current directory.
    Fuzzy {
        /// Create symlinks to selected files in current directory
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
/// - The fuzzy search process fails
pub fn run_notes_command(
    cmd: NotesCommand,
    db_path: &std::path::Path,
    rt: &tokio::runtime::Handle,
) -> Result<(), Report<RunError>> {
    rt.block_on(async {
        let services = Services::new(&db_path.to_string_lossy(), rt.clone())
            .await
            .change_context(RunError)?;

        match cmd {
            NotesCommand::Add { paths } => {
                add::handle_add_command(&services, paths).await.change_context(RunError)
            }
            NotesCommand::Search { query, symlink } => {
                search::handle_search_command(&services, &query, symlink).await.change_context(RunError)
            }
            NotesCommand::Fuzzy { symlink } => {
                fuzzy::handle_fuzzy_command(&services, symlink).await.change_context(RunError)
            }
        }
    })
}
