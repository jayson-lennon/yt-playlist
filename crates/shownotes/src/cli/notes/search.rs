use error_stack::{Report, ResultExt};

use crate::feat::note_db::NoteDb;
use crate::services::Services;
use super::symlink;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct SearchError;

pub async fn handle_search_command(
    services: &Services,
    query: &str,
    create_symlinks: bool,
) -> Result<(), Report<SearchError>> {
    let results: Vec<_> = services
        .db
        .search_notes(query)
        .await
        .change_context(SearchError)?
        .into_iter()
        .collect();

    for path in &results {
        println!("{path}");
    }

    if create_symlinks {
        symlink::create_symlinks_for_paths(&results)
            .change_context(SearchError)?;
    }

    Ok(())
}
