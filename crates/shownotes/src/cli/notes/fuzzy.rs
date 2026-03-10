use std::sync::Arc;

use error_stack::{Report, ResultExt};

use crate::feat::fuzzy_search::{FuzzySearchService, backend::SkimBackend};
use crate::feat::note_db::NoteDb;
use crate::services::Services;
use super::symlink;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct FuzzyError;

pub async fn handle_fuzzy_command(
    services: &Services,
    create_symlinks: bool,
) -> Result<(), Report<FuzzyError>> {
    let fuzzy_search = FuzzySearchService::new(Arc::new(SkimBackend));

    let notes = services
        .db
        .get_all_notes_with_paths()
        .await
        .change_context(FuzzyError)?;

    if notes.is_empty() {
        return Ok(());
    }

    let result = fuzzy_search.search(&notes).change_context(FuzzyError)?;

    for path in &result.selected_paths {
        println!("{path}");
    }

    if create_symlinks {
        symlink::create_symlinks_for_paths(&result.selected_paths)
            .change_context(FuzzyError)?;
    }

    Ok(())
}
