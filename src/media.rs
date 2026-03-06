use std::{path::Path, sync::Arc, time::Duration};

use derive_more::Debug;
use error_stack::{Report, ResultExt};
use wherror::Error;

#[derive(Debug, Error)]
#[error(debug)]
pub struct MediaError;

#[allow(clippy::missing_errors_doc)]
pub trait MediaQueryBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>>;
}

#[derive(Debug, Clone)]
pub struct MediaQuery {
    #[debug("backend<{}>", self.backend.name())]
    backend: Arc<dyn MediaQueryBackend>,
}

#[allow(clippy::missing_errors_doc)]
impl MediaQuery {
    pub fn new(backend: Arc<dyn MediaQueryBackend>) -> Self {
        Self { backend }
    }

    pub fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        self.backend.get_duration(path)
    }
}

pub struct FfprobeBackend;

impl MediaQueryBackend for FfprobeBackend {
    fn name(&self) -> &'static str {
        "ffprobe"
    }

    fn get_duration(&self, path: &Path) -> Result<Duration, Report<MediaError>> {
        let info = ffprobe::ffprobe(path).change_context(MediaError)?;
        info.format
            .get_duration()
            .ok_or_else(|| Report::new(MediaError))
            .attach("no duration in ffprobe output")
    }
}
