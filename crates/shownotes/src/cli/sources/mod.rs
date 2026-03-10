use std::path::PathBuf;

use clap::Subcommand;
use error_stack::{Report, ResultExt};

use crate::command::{format_output, execute, Command};
use crate::services::Services;

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
pub fn run_sources_command(
    cmd: SourcesCommands,
    db_path: &std::path::Path,
    rt: &tokio::runtime::Handle,
) -> Result<(), Report<RunError>> {
    rt.block_on(async {
        let services = Services::new(&db_path.to_string_lossy(), rt.clone())
            .await
            .change_context(RunError)?;

        let command = match cmd {
            SourcesCommands::Add { path, url } => Command::SourcesAdd { path, url },
            SourcesCommands::List { path } => Command::SourcesList { path },
            SourcesCommands::Edit { path } => Command::SourcesEdit { path },
        };

        let result = execute(&services, command)
            .await
            .change_context(RunError)?;

        println!("{}", format_output(&result));
        Ok(())
    })
}
