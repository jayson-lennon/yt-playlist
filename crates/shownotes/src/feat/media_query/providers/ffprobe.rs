use std::{path::Path, time::Duration};

use error_stack::{Report, ResultExt};

use super::super::{MediaError, MediaQuery};

pub struct Ffprobe;

impl MediaQuery for Ffprobe {
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
