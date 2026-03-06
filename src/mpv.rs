use std::{path::Path, sync::Arc};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use mpvipc::{Mpv, MpvCommand, PlaylistAddOptions};
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
    mpv: Option<Mpv>,
}

impl MpvipcBackend {
    pub fn new(socket_path: &Path) -> Self {
        let socket = socket_path.to_string_lossy().into_owned();
        let mpv = Mpv::connect(&socket).ok();
        Self { mpv }
    }
}

impl MpvBackend for MpvipcBackend {
    fn name(&self) -> &'static str {
        "mpvipc"
    }

    fn load_file(&self, path: &Path) -> Result<(), Report<MpvError>> {
        let mpv = self
            .mpv
            .as_ref()
            .ok_or_else(|| Report::new(MpvError))
            .attach("mpv not connected")?;
        mpv.run_command(MpvCommand::LoadFile {
            file: path.to_string_lossy().into_owned(),
            option: PlaylistAddOptions::Replace,
        })
        .change_context(MpvError)?;
        Ok(())
    }
}
