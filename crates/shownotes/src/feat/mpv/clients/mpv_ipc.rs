// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
