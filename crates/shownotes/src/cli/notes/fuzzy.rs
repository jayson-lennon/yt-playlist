use error_stack::{Report, ResultExt};

use crate::feat::commands::fuzzy_notes;
use crate::services::Services;

#[derive(Debug, wherror::Error)]
#[error(debug)]
pub struct FuzzyError;

pub async fn handle_fuzzy_command(
    services: &Services,
    create_symlinks: bool,
) -> Result<(), Report<FuzzyError>> {
    let result = fuzzy_notes(services, create_symlinks)
        .await
        .change_context(FuzzyError)?;

    for path in &result.paths {
        println!("{path}");
    }

    Ok(())
}
