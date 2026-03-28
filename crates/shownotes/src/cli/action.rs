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

use std::path::Path;

use clap::Subcommand;
use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::app::App;
use crate::command::{format_output, Command};

use super::RunError;

/// CLI subcommands for file actions.
///
/// Each variant represents a specific action that can be performed on a file,
/// such as loading it in an external application via IPC.
#[derive(Subcommand)]
pub enum ActionCommands {
    /// Load a file in mpv via IPC.
    Mpv {
        /// File path to open
        path: std::path::PathBuf,

        /// mpv socket path
        #[arg(long, default_value = "/tmp/mpvsocket")]
        socket: std::path::PathBuf,
    },
}

/// # Errors
///
/// Returns an error if the MPV load command fails.
pub fn run_action_mpv(path: &Path, socket: &Path, app: &mut App) -> Result<(), Report<RunError>> {
    let canonical_path = CanonicalPath::from_path(path).change_context(RunError)?;
    let command = Command::MpvLoad {
        path: canonical_path,
        socket: socket.to_path_buf(),
    };

    let result = app.execute(command).change_context(RunError)?;
    println!("{}", format_output(&result));
    Ok(())
}
