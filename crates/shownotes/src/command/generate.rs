use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::system_ctx::SystemCtx;

use super::CommandError;

pub async fn execute(
    ctx: &SystemCtx,
    working_directory: &CanonicalPath,
    format: &str,
) -> Result<String, Report<CommandError>> {
    let working_directory = CanonicalPath::from_path(working_directory.as_path())
        .map_err(|_| Report::new(CommandError))
        .attach("Failed to canonicalize working directory")?;
    let playlist_data = ctx.services.storage.load(&working_directory).await.change_context(CommandError)?;

    crate::feat::generate_show_notes(&playlist_data, &ctx.services.sources, format)
        .await
        .change_context(CommandError)
}
