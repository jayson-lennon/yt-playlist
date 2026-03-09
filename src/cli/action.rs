use std::path::Path;

use clap::Subcommand;
use error_stack::{Report, ResultExt};

use crate::feat::mpv::{MpvClient, MpvIpc};

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

/// Loads a file in mpv via IPC.
///
/// # Errors
///
/// Returns an error if the file cannot be loaded via the mpv IPC socket.
pub fn run_action_mpv(path: &Path, socket: &Path) -> Result<(), Report<RunError>> {
    let backend = MpvIpc::new(socket);
    backend.load_file(path).change_context(RunError)?;
    println!("Loaded: {}", path.display());
    Ok(())
}
