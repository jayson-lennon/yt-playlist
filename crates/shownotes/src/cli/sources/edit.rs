use std::path::PathBuf;
use error_stack::{Report, ResultExt};
use crate::services::Services;
use crate::feat::commands::edit_sources;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct EditError;

pub async fn handle_edit_command(
    services: &Services,
    path: PathBuf,
) -> Result<(), Report<EditError>> {
    edit_sources(services, &path).await.change_context(EditError)
}
