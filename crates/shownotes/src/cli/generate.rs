use std::path::Path;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::app::App;
use crate::command::{format_output, Command};

use super::RunError;

/// # Errors
///
/// Returns an error if note generation fails.
pub fn run_generate(format: &str, path: &Path, app: &mut App) -> Result<(), Report<RunError>> {
    let canonical_path = CanonicalPath::from_path(path).change_context(RunError)?;
    let command = Command::GenerateNotes {
        format: format.to_string(),
        working_directory: canonical_path,
    };

    let result = app.execute(command).change_context(RunError)?;
    println!("{}", format_output(&result));
    Ok(())
}
