use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use mpvipc::{Mpv, MpvCommand, PlaylistAddOptions};
use sysinfo::System;
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct MpvError;

pub trait MpvClient: Send + Sync {
    /// Returns the name identifier for this backend implementation.
    fn name(&self) -> &'static str;

    /// Loads a single media file into mpv, replacing the current playlist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be loaded into mpv.
    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>>;

    /// Loads multiple media files as a playlist into mpv, replacing the current playlist.
    ///
    /// # Errors
    ///
    /// Returns an error if the playlist cannot be loaded into mpv.
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
    ///
    /// Returns an error if the file cannot be loaded by the backend.
    pub fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        self.backend.load_file(path)
    }

    /// # Errors
    ///
    /// Returns an error if the playlist cannot be loaded by the backend.
    pub fn load_playlist(&self, paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
        self.backend.load_playlist(paths)
    }
}

pub struct MpvIpc {
    socket_path: String,
}

impl MpvIpc {
    pub fn new(socket_path: &Path) -> Self {
        let socket = socket_path.to_string_lossy().into_owned();
        Self {
            socket_path: socket,
        }
    }
}

impl MpvClient for MpvIpc {
    fn name(&self) -> &'static str {
        "mpvipc"
    }

    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        let mpv = Mpv::connect(&self.socket_path)
            .change_context(MpvError)
            .attach("failed to connect to mpv")?;
        mpv.run_command(MpvCommand::LoadFile {
            file: path.to_string_lossy().into_owned(),
            option: PlaylistAddOptions::Replace,
        })
        .change_context(MpvError)?;
        Ok(())
    }

    fn load_playlist(&self, paths: &[PathBuf]) -> Result<(), Report<MpvError>> {
        let temp_dir = std::env::temp_dir();
        let playlist_path = temp_dir.join("shownotes-temp.m3u");
        let file = File::create(&playlist_path)
            .change_context(MpvError)
            .attach("failed to create temp playlist file")?;
        let mut writer = BufWriter::new(file);
        for path in paths {
            writeln!(writer, "{}", path.to_string_lossy())
                .change_context(MpvError)
                .attach("failed to write to temp playlist file")?;
        }
        writer.flush().change_context(MpvError)?;
        let mpv = Mpv::connect(&self.socket_path)
            .change_context(MpvError)
            .attach("failed to connect to mpv")?;
        mpv.run_command(MpvCommand::LoadList {
            file: playlist_path.to_string_lossy().into_owned(),
            option: PlaylistAddOptions::Replace,
        })
        .change_context(MpvError)?;
        Ok(())
    }
}

pub trait MpvLauncher: Send + Sync {
    /// Returns the name identifier for this launcher implementation.
    fn name(&self) -> &'static str;

    /// Checks whether an mpv process is currently running with the specified socket path.
    fn is_running(&self, socket_path: &str) -> bool;

    /// Spawns a new mpv process configured with the specified IPC socket path.
    ///
    /// # Errors
    ///
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
    ///
    /// Returns an error if mpv cannot be spawned by the backend.
    pub fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>> {
        self.backend.spawn(socket_path)
    }
}

pub struct RealMpvLauncher;

impl MpvLauncher for RealMpvLauncher {
    fn name(&self) -> &'static str {
        "real"
    }

    fn is_running(&self, socket_path: &str) -> bool {
        is_mpv_running_with_socket(socket_path)
    }

    fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>> {
        spawn_mpv(socket_path)
    }
}

fn is_mpv_running_with_socket(socket_path: &str) -> bool {
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

fn spawn_mpv(socket_path: &str) -> Result<(), Report<MpvError>> {
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
