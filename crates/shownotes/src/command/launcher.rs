use std::path::Path;

use error_stack::{Report, ResultExt};

use super::CommandError;
use crate::feat::launcher::LaunchResult;
use crate::services::Services;

pub fn launch(
    services: &Services,
    path: &Path,
    command: Option<&str>,
    socket_path: &str,
) -> Result<LaunchResult, Report<CommandError>> {
    services
        .file_launcher
        .launch(path, command, socket_path)
        .change_context(CommandError)
        .attach("failed to launch file")
}
