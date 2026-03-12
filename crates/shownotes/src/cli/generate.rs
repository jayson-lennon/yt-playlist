use std::path::Path;

use error_stack::{Report, ResultExt};
use marked_path::CanonicalPath;

use crate::command::{format_output, execute, Command};
use crate::services::Services;

use super::RunError;

/// # Errors
///
/// Returns an error if note generation fails.
pub fn run_generate(
    format: &str,
    path: &Path,
    db_path: &Path,
    rt: &tokio::runtime::Handle,
) -> Result<(), Report<RunError>> {
    rt.block_on(async {
        let services = Services::new(&db_path.to_string_lossy(), rt.clone())
            .await
            .change_context(RunError)?;

        let canonical_path = CanonicalPath::from_path(path).change_context(RunError)?;
        let command = Command::GenerateNotes {
            format: format.to_string(),
            working_directory: canonical_path,
        };

        let result = execute(&services, command)
            .await
            .change_context(RunError)?;

        println!("{}", format_output(&result));
        Ok(())
    })
}
