use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use super::CommandError;
use crate::feat::launcher::LaunchResult;
use crate::system_ctx::SystemCtx;

pub fn launch(
    ctx: &SystemCtx,
    path: &CanonicalPath,
    command: Option<&str>,
    socket_path: &str,
) -> Result<LaunchResult, Report<CommandError>> {
    ctx.services
        .file_launcher
        .launch(path.as_path(), command, socket_path)
        .change_context(CommandError)
        .attach("failed to launch file")
}
