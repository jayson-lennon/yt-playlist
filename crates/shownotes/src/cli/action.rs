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
