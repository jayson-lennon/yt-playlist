use std::path::PathBuf;

use clap::Subcommand;
use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::app::App;
use crate::command::{format_output, Command};

use super::RunError;

#[derive(Subcommand)]
pub enum SourcesCommands {
    /// Add a source URL to a file
    Add {
        /// File path
        path: PathBuf,
        /// Source URL
        url: String,
    },

    /// List source URLs for a file
    List {
        /// File path
        path: PathBuf,
    },

    /// Edit source URLs for a file in $EDITOR
    Edit {
        /// File path
        path: PathBuf,
    },
}

/// Runs a sources command.
///
/// # Errors
///
/// Returns an error if:
/// - The database cannot be accessed
/// - Path resolution fails
/// - The editor fails to open
pub fn run_sources_command(cmd: SourcesCommands, app: &mut App) -> Result<(), Report<RunError>> {
    let command = match cmd {
        SourcesCommands::Add { path, url } => {
            let canonical = CanonicalPath::from_path(&path).change_context(RunError)?;
            Command::SourcesAdd {
                path: canonical,
                url,
            }
        }
        SourcesCommands::List { path } => {
            let canonical = CanonicalPath::from_path(&path).change_context(RunError)?;
            Command::SourcesList { path: canonical }
        }
        SourcesCommands::Edit { path } => {
            let canonical = CanonicalPath::from_path(&path).change_context(RunError)?;
            Command::SourcesEdit { path: canonical }
        }
    };

    let result = app.execute(command).change_context(RunError)?;
    println!("{}", format_output(&result));
    Ok(())
}
