use std::{
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
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
    mpv: Mutex<Option<Mpv>>,
}

impl MpvipcBackend {
    pub fn new(socket_path: &Path) -> Self {
        let socket = socket_path.to_string_lossy().into_owned();
        let mpv = Mpv::connect(&socket).ok();
        Self {
            socket_path: socket,
            mpv: Mutex::new(mpv),
        }
    }

    fn get_or_connect(&self) -> Result<Mpv, Report<MpvError>> {
        let mut guard = self.mpv.lock().map_err(|_| Report::new(MpvError))?;
        if let Some(mpv) = guard.as_ref() {
            return Ok(mpv.clone());
        }
        let mpv = Mpv::connect(&self.socket_path)
            .change_context(MpvError)
            .attach("failed to connect to mpv")?;
        *guard = Some(mpv.clone());
        Ok(mpv)
    }
}

impl MpvBackend for MpvipcBackend {
    fn name(&self) -> &'static str {
        "mpvipc"
    }

    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        let mpv = self.get_or_connect()?;
        mpv.run_command(MpvCommand::LoadFile {
            file: path.to_string_lossy().into_owned(),
            option: PlaylistAddOptions::Replace,
        })
        .change_context(MpvError)?;
        Ok(())
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

/// Spawns a new MPV process with the given socket path.
///
/// # Errors
///
/// Returns an error if the MPV process fails to spawn.
pub fn spawn_mpv(socket_path: &str) -> Result<(), Report<MpvError>> {
    Command::new("mpv")
        .args([
            "--keep-open=yes",
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
