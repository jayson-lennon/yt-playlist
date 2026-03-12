
use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use super::CommandError;
use crate::feat::launcher::LaunchResult;
use crate::services::Services;

pub fn launch(
    services: &Services,
    path: &CanonicalPath,
    command: Option<&str>,
    socket_path: &str,
) -> Result<LaunchResult, Report<CommandError>> {
    services
        .file_launcher
        .launch(path.as_path(), command, socket_path)
        .change_context(CommandError)
        .attach("failed to launch file")
}
