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

use std::path::Path;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use super::CommandError;
use crate::feat::mpv::MpvIpc;
use crate::feat::MpvClient;
use crate::system_ctx::SystemCtx;

pub fn load(socket: &Path, path: &CanonicalPath) -> Result<(), Report<CommandError>> {
    let backend = MpvIpc::new(socket);
    backend
        .load_file(path.as_path())
        .change_context(CommandError)
}

pub fn load_playlist(ctx: &SystemCtx, paths: &[CanonicalPath]) -> Result<(), Report<CommandError>> {
    let paths: Vec<std::path::PathBuf> = paths.iter().map(|p| p.as_path().to_path_buf()).collect();
    ctx.services
        .mpv
        .load_playlist(&paths)
        .change_context(CommandError)
        .attach("failed to load playlist into mpv")
}

pub fn spawn(ctx: &SystemCtx, socket_path: &str) -> Result<bool, Report<CommandError>> {
    if ctx.services.mpv_launcher.is_running(socket_path) {
        return Ok(true);
    }
    ctx.services
        .mpv_launcher
        .spawn(socket_path)
        .change_context(CommandError)
        .attach("failed to spawn mpv")?;
    Ok(false)
}

pub fn toggle_play(ctx: &SystemCtx) -> Result<(), Report<CommandError>> {
    ctx.services
        .mpv
        .toggle_play()
        .change_context(CommandError)
        .attach("failed to toggle play/pause in mpv")
}
