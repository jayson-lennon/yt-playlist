// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::path::PathBuf;

use clap::Subcommand;
use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::app::App;
use crate::command::{format_output, Command};

use super::RunError;

/// CLI subcommands for source URL management.
///
/// Source URLs track the provenance of files (e.g., where a video was
/// downloaded from). Each variant provides a different operation for
/// managing these associations.
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
