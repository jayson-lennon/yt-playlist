use std::path::Path;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use super::CommandError;
use crate::feat::mpv::MpvIpc;
use crate::feat::MpvClient;
use crate::services::Services;

pub fn load(socket: &Path, path: &CanonicalPath) -> Result<(), Report<CommandError>> {
    let backend = MpvIpc::new(socket);
    backend
        .load_file(path.as_path())
        .change_context(CommandError)
}

pub fn load_playlist(
    services: &Services,
    paths: &[CanonicalPath],
) -> Result<(), Report<CommandError>> {
    let paths: Vec<std::path::PathBuf> = paths.iter().map(|p| p.as_path().to_path_buf()).collect();
    services
        .mpv
        .load_playlist(&paths)
        .change_context(CommandError)
        .attach("failed to load playlist into mpv")
}

pub fn spawn(services: &Services, socket_path: &str) -> Result<bool, Report<CommandError>> {
    if services.mpv_launcher.is_running(socket_path) {
        return Ok(true);
    }
    services
        .mpv_launcher
        .spawn(socket_path)
        .change_context(CommandError)
        .attach("failed to spawn mpv")?;
    Ok(false)
}
