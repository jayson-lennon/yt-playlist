use std::path::PathBuf;
use error_stack::{Report, ResultExt};
use crate::services::Services;
use crate::feat::sources::SourceDb;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct AddError;

pub async fn handle_add_command(
    services: &Services,
    path: PathBuf,
    url: String,
) -> Result<(), Report<AddError>> {
    let (_, file_path_id) = super::common::resolve_and_get_file_path(services, &path)
        .await
        .change_context(AddError)?;

    let mut existing = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(AddError)?
        .into_iter()
        .map(|s| s.source_url)
        .collect::<Vec<_>>();
    existing.push(url);

    services
        .sources
        .set_sources(file_path_id, &existing)
        .await
        .change_context(AddError)?;

    println!("Added source to: {}", path.display());
    Ok(())
}
