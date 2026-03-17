use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use error_stack::{Report, ResultExt};
use mpvipc::{Mpv, MpvCommand, PlaylistAddOptions};

use crate::feat::mpv::{MpvClient, MpvError};

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

    fn toggle_play(&self) -> Result<(), Report<MpvError>> {
        let mpv = Mpv::connect(&self.socket_path)
            .change_context(MpvError)
            .attach("failed to connect to mpv")?;
        mpv.toggle().change_context(MpvError)?;
        Ok(())
    }
}
