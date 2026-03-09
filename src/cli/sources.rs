use std::path::PathBuf;

use clap::Subcommand;
use error_stack::{Report, ResultExt};

use crate::{
    feat::{ExternalEditor, NoteDb, PathResolver, sources::SourceDb},
    services::Services,
};

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
) -> Result<(), Report<RunError>> {
    let rt = tokio::runtime::Runtime::new().change_context(RunError)?;
    rt.block_on(async { run_sources_command_async(cmd, db_path).await })
}

#[allow(clippy::too_many_lines)]
async fn run_sources_command_async(
    cmd: SourcesCommands,
    db_path: &std::path::Path,
) -> Result<(), Report<RunError>> {
    let services = Services::new(&db_path.to_string_lossy())
        .await
        .change_context(RunError)?;

    match cmd {
        SourcesCommands::Add { path, url } => {
            let resolved = services
                .path_resolver
                .resolve(&path)
                .await
                .change_context(RunError)?;

            let path_str = resolved.to_string_lossy();
            let file_path_id = services
                .db
                .get_or_create_file_path(&path_str)
                .await
                .change_context(RunError)?;

            let mut existing = services
                .sources
                .get_sources(file_path_id)
                .await
                .change_context(RunError)?
                .into_iter()
                .map(|s| s.source_url)
                .collect::<Vec<_>>();
            existing.push(url);

            services
                .sources
                .set_sources(file_path_id, &existing)
                .await
                .change_context(RunError)?;

            println!("Added source to: {}", path.display());
        }
        SourcesCommands::List { path } => {
            let resolved = services
                .path_resolver
                .resolve(&path)
                .await
                .change_context(RunError)?;

            let path_str = resolved.to_string_lossy();
            let file_path_id = services
                .db
                .get_or_create_file_path(&path_str)
                .await
                .change_context(RunError)?;

            let sources = services
                .sources
                .get_sources(file_path_id)
                .await
                .change_context(RunError)?;

            if sources.is_empty() {
                println!("No sources found for: {}", path.display());
            } else {
                for source in sources {
                    println!("{}", source.source_url);
                }
            }
        }
        SourcesCommands::Edit { path } => {
            let resolved = services
                .path_resolver
                .resolve(&path)
                .await
                .change_context(RunError)?;

            let path_str = resolved.to_string_lossy();
            let file_path_id = services
                .db
                .get_or_create_file_path(&path_str)
                .await
                .change_context(RunError)?;

            let existing = services
                .sources
                .get_sources(file_path_id)
                .await
                .change_context(RunError)?;
            let initial_content = existing
                .iter()
                .map(|s| s.source_url.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            if let Some(new_content) = services
                .editor
                .open(&initial_content)
                .await
                .change_context(RunError)?
            {
                let urls: Vec<String> = new_content.lines().map(ToString::to_string).collect();
                services
                    .sources
                    .set_sources(file_path_id, &urls)
                    .await
                    .change_context(RunError)?;
                println!("Updated sources for: {}", path.display());
            }
        }
    }

    Ok(())
}
