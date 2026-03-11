use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use sysinfo::System;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct MpvError;

pub trait MpvClient: Send + Sync {
    fn name(&self) -> &'static str;

    /// # Errors
    /// Returns an error if the file cannot be loaded in mpv.
    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>>;

    /// # Errors
    /// Returns an error if the playlist cannot be loaded in mpv.
    fn load_playlist(&self, paths: &[PathBuf]) -> Result<(), Report<MpvError>>;
}

#[derive(Debug, Clone)]
pub struct MpvClientService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MpvClient>,
}

impl MpvClientService {
    pub fn new(backend: Arc<dyn MpvClient>) -> Self {
        Self { backend }
    }

    /// # Errors
    /// Returns an error if the file cannot be loaded in mpv.
    pub fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        self.backend.load_file(path)
    }

    /// # Errors
    /// Returns an error if the playlist cannot be loaded in mpv.
    pub fn load_playlist(&self, paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
        self.backend.load_playlist(paths)
    }
}

pub trait MpvLauncher: Send + Sync {
    fn name(&self) -> &'static str;

    fn is_running(&self, socket_path: &str) -> bool;

    /// # Errors
    /// Returns an error if mpv cannot be spawned.
    fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>>;
}

#[derive(Debug, Clone)]
pub struct MpvLauncherService {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MpvLauncher>,
}

impl MpvLauncherService {
    pub fn new(backend: Arc<dyn MpvLauncher>) -> Self {
        Self { backend }
    }

    pub fn is_running(&self, socket_path: &str) -> bool {
        self.backend.is_running(socket_path)
    }

    /// # Errors
    /// Returns an error if mpv cannot be spawned.
    pub fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>> {
        self.backend.spawn(socket_path)
    }
}

pub fn is_mpv_running_with_socket(socket_path: &str) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();

    for process in sys.processes().values() {
        let name = process.name().to_string_lossy();
        if name == "mpv" {
            for arg in process.cmd() {
                let arg_str = arg.to_string_lossy();
                if arg_str.contains("--input-ipc-server=") && arg_str.contains(socket_path) {
                    return true;
                }
            }
        }
    }
    false
}

/// # Errors
/// Returns an error if mpv cannot be spawned.
pub fn spawn_mpv(socket_path: &str) -> Result<(), Report<MpvError>> {
    Command::new("mpv")
        .args([
            "--keep-open=always",
            "--idle",
            &format!("--input-ipc-server={socket_path}"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .change_context(MpvError)
        .attach("failed to spawn mpv")?;
    Ok(())
}

pub mod clients;

pub use clients::{MpvIpc, RealMpvLauncher};
