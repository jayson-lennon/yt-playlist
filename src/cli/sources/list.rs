use std::path::PathBuf;
use error_stack::{Report, ResultExt};
use crate::services::Services;
use crate::feat::sources::SourceDb;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct ListError;

pub async fn handle_list_command(
    services: &Services,
    path: PathBuf,
) -> Result<(), Report<ListError>> {
    let (_resolved, file_path_id) = super::common::resolve_and_get_file_path(services, &path)
        .await
        .change_context(ListError)?;

    let sources = services
        .sources
        .get_sources(file_path_id)
        .await
        .change_context(ListError)?;

    if sources.is_empty() {
        println!("No sources found for: {}", path.display());
    } else {
        for source in sources {
            println!("{}", source.source_url);
        }
    }

    Ok(())
}
