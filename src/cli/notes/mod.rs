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
    Add { paths: Vec<PathBuf> },
    Search {
        query: String,
        #[arg(long)]
        symlink: bool,
    },
    Fuzzy {
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
