use std::path::Path;

use clap::Subcommand;
use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::command::{format_output, execute, Command};
use crate::services::Services;

use super::RunError;

#[derive(Subcommand)]
pub enum ActionCommands {
    /// Load a file in mpv via IPC
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
pub fn run_action_mpv(
    path: &Path,
    socket: &Path,
    db_path: &Path,
    rt: &tokio::runtime::Handle,
) -> Result<(), Report<RunError>> {
    rt.block_on(async {
        let services = Services::new(&db_path.to_string_lossy(), rt.clone())
            .await
            .change_context(RunError)?;

        let canonical_path = CanonicalPath::from_path(path).change_context(RunError)?;
        let command = Command::MpvLoad {
            path: canonical_path,
            socket: socket.to_path_buf(),
        };

        let result = execute(&services, command)
            .await
            .change_context(RunError)?;

        println!("{}", format_output(&result));
        Ok(())
    })
}
