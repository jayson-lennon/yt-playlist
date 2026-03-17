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
