use std::{
    path::Path,
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

#[allow(clippy::missing_errors_doc)]
pub trait MpvBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>>;
}

#[derive(Debug, Clone)]
pub struct MpvClient {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MpvBackend>,
}

#[allow(clippy::missing_errors_doc)]
impl MpvClient {
    pub fn new(backend: Arc<dyn MpvBackend>) -> Self {
        Self { backend }
    }

    pub fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        self.backend.load_file(path)
    }
}

pub struct MpvipcBackend {
    socket_path: String,
}

impl MpvipcBackend {
    pub fn new(socket_path: &Path) -> Self {
        let socket = socket_path.to_string_lossy().into_owned();
        Self {
            socket_path: socket,
        }
    }
}

impl MpvBackend for MpvipcBackend {
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
}

pub trait MpvLauncher: Send + Sync {
    fn is_running(&self, socket_path: &str) -> bool;
    fn spawn(&self, socket_path: &str) -> Result<(), Report<MpvError>>;
}

pub struct RealMpvLauncher;

impl MpvLauncher for RealMpvLauncher {
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
